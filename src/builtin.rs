use datum::{Datum, Procedure};
use environment::Environment;
use error::RuntimeError;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use vm::{Instruction, DefineType};

pub fn get_builtins() -> Vec<(&'static str, Datum)>
{
    vec![
        ("begin", Datum::special(special_form_begin)),
        ("define", Datum::special(special_form_define)),
        ("define-syntax", Datum::special(special_form_define_syntax)),
        ("eval", Datum::special(special_form_eval)),
        ("if", Datum::special(special_form_if)),
        ("lambda", Datum::special(special_form_lambda)),
        ("letrec", Datum::special(special_form_letrec)),
        ("quote", Datum::special(special_form_quote)),
        ("set!", Datum::special(special_form_set)),
        ("syntax-rules", Datum::special(special_form_syntax_rules)),

        ("+", Datum::native(native_add)),
        ("-", Datum::native(native_subtract)),
        ("*", Datum::native(native_multiply)),
        ("=", Datum::native(native_equals)),
        ("append", Datum::native(native_append)),
        ("car", Datum::native(native_car)),
        ("cdr", Datum::native(native_cdr)),
        ("cons", Datum::native(native_cons)),
        ("eq?", Datum::native(native_eqv_p)), // same as eqv?
        ("equal?", Datum::native(native_equal_p)),
        ("eqv?", Datum::native(native_eqv_p)),
        ("hash-ref", Datum::native(native_hash_ref)),
        ("hash-set!", Datum::native(native_hash_set)),
        ("length", Datum::native(native_length)),
        ("list", Datum::native(native_list)),
        ("list->string", Datum::native(native_list_to_string)),
        ("make-hash-table", Datum::native(native_make_hash_table)),
        ("null?", Datum::native(native_null_p)),
        ("reverse", Datum::native(native_reverse)),
        ("string=?", Datum::native(native_string_equal_p)),
        ("string-append", Datum::native(native_string_append)),
        ("string-contains", Datum::native(native_string_contains)),
        ("string-length", Datum::native(native_string_length)),
        ("string-prefix?", Datum::native(native_string_prefix_p)),
        ("string-split", Datum::native(native_string_split)),
        ("string->list", Datum::native(native_string_to_list)),
        ("string->number", Datum::native(native_string_to_number)),
        ("string->symbol", Datum::native(native_string_to_symbol)),
        ("substring", Datum::native(native_substring)),
        ("symbol->string", Datum::native(native_symbol_to_string)),

        ("boolean?", Datum::native(native_boolean_p)),
        ("char?", Datum::native(native_char_p)),
        ("number?", Datum::native(native_number_p)),
        ("pair?", Datum::native(native_pair_p)),
        ("procedure?", Datum::native(native_procedure_p)),
        ("string?", Datum::native(native_string_p)),
        ("symbol?", Datum::native(native_symbol_p)),
        ("vector?", Datum::native(native_vector_p)),
    ]
}

fn special_form_begin(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args >= 1);
    let mut instructions = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let last = i == args.len() - 1;
        instructions.push(
            Instruction::PushValue(arg.clone()));
        instructions.push(
            Instruction::Evaluate(env.clone(), last));
        if !last {
            instructions.push(Instruction::PopValue);
        }
    }

    Ok(instructions)
}

fn special_form_define(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args >= 2);
    let usage_str =
        format!("Usage: (define variable value) OR (define (proc formals) body ...)");
    match args[0] {
        Datum::Symbol(ref name) => {
            expect_args!(args == 2);
            let instructions = vec![
                Instruction::PushValue(args[1].clone()),
                Instruction::Evaluate(env.clone(), false),
                Instruction::Define(env.clone(), name.clone(),
                    DefineType::Define),
                // Return value is unspecified in the spec.
                Instruction::PushValue(Datum::EmptyList)
            ];
            Ok(instructions)
        },
        Datum::Pair(ref car, ref cdr) => {
            match **car {
                Datum::Symbol(ref name) => {
                    let formals = *cdr.clone();
                    let body: Vec<_> =
                        args[1..].iter().map(|d| d.clone()).collect();
                    let mut lambda_args = vec![formals];
                    lambda_args.extend(body);
                    let mut instructions = try!(
                        special_form_lambda(env.clone(), &lambda_args));
                    instructions.push(
                        Instruction::Define(env.clone(), name.clone(),
                            DefineType::Define));
                    instructions.push(
                        Instruction::PushValue(Datum::EmptyList));
                    Ok(instructions)
                },
                _ => runtime_error!("{}", &usage_str)
            }
        },
        _ => runtime_error!("{}", &usage_str)
    }
}

fn special_form_define_syntax(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args == 2);
    let name = try_unwrap_arg!(args[0] => Symbol).clone();
    let instructions = vec![
        Instruction::PushValue(args[1].clone()),
        Instruction::Evaluate(env.clone(), false),
        Instruction::Define(env.clone(), name, DefineType::DefineSyntax),
        // Return value is unspecified in the spec.
        Instruction::PushValue(Datum::EmptyList)
    ];
    Ok(instructions)
}

fn special_form_eval(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args == 1);
    let instructions = vec![
        Instruction::PushValue(args[0].clone()),
        Instruction::Evaluate(env.clone(), false),
        Instruction::Evaluate(env.clone(), false),
    ];
    Ok(instructions)
}

fn special_form_if(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    if args.len() != 2 && args.len() != 3 {
        runtime_error!("Expected 2 or 3 args");
    }

    let mut instructions = vec![
        Instruction::PushValue(args[0].clone()),
        Instruction::Evaluate(env.clone(), false),
        Instruction::JumpIfFalse(4),
        Instruction::PushValue(args[1].clone()),
        Instruction::Evaluate(env.clone(), true),
        Instruction::Return
    ];
    if args.len() == 3 {
        instructions.push(Instruction::PushValue(args[2].clone()));
        instructions.push(Instruction::Evaluate(env.clone(), true));
    } else {
        // Unspecified in the spec.
        instructions.push(Instruction::PushValue(Datum::Boolean(false)));
    }

    Ok(instructions)
}

fn special_form_lambda(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args >= 2);
    let (arg_names, rest_name) = match args[0] {
        Datum::Symbol(ref s) => (Vec::new(), Some(s.clone())),
        ref d @ Datum::Pair(..) => {
            let (formals, is_proper) = d.as_vec();
            let mut arg_names = Vec::new();
            for formal in formals {
                arg_names.push(match formal {
                    Datum::Symbol(s) => s,
                    _ => runtime_error!("Expected list or symbol list for formals")
                });
            }
            if is_proper {
                (arg_names, None)
            } else {
                let loc = arg_names.len() - 1;
                let rest_name = arg_names.split_off(loc);
                (arg_names, Some(rest_name.first().unwrap().clone()))
            }
        },
        Datum::EmptyList => (Vec::new(), None),
        _ => runtime_error!("Expected symbol or symbol list for formals")
    };
    let body = Vec::from(&args[1..]);
    let lambda = Datum::scheme(arg_names, rest_name, body, env.clone());
    Ok(vec![Instruction::PushValue(lambda)])
}

fn special_form_letrec(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    let usage_str =
        format!("Usage: (letrec ((variable init) ...) body ...)");
    if args.len() < 2 { runtime_error!("{}", &usage_str); }

    // Parse the bindings and add instructions for evaluating the initial
    // values to define within a sub-environment.
    let mut instructions = Vec::new();
    let let_env = Rc::new(RefCell::new(Environment::with_parent(env.clone())));
    let bindings = try_or_runtime_error!(args[0].to_vec(), "{}", &usage_str);
    for binding in bindings {
        let mut parts =
            try_or_runtime_error!(binding.to_vec(), "{}", &usage_str);
        if parts.len() != 2 { runtime_error!("{}", &usage_str); }
        let init = parts.remove(1);
        let variable = parts.remove(0);
        let var_name = match variable {
            Datum::Symbol(ref s) => s.to_string(),
            _ => runtime_error!("{}", &usage_str)
        };
        instructions.push(Instruction::PushValue(init));
        instructions.push(Instruction::Evaluate(let_env.clone(), false));
        instructions.push(
            Instruction::Define(let_env.clone(), var_name, DefineType::Define));
    }

    // Add the instructions for evaluating the body within the sub-environment.
    for (i, arg) in args.iter().skip(1).enumerate() {
        let last = i == args.len() - 2;
        instructions.push(Instruction::PushValue(arg.clone()));
        instructions.push(Instruction::Evaluate(let_env.clone(), last));
        if !last {
            instructions.push(Instruction::PopValue);
        }
    }
    Ok(instructions)
}

fn special_form_quote(_: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args == 1);
    Ok(vec![Instruction::PushValue(args[0].clone())])
}

fn special_form_set(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    expect_args!(args == 2);
    let name = try_unwrap_arg!(args[0] => Symbol);
    let instructions = vec![
        Instruction::PushValue(args[1].clone()),
        Instruction::Evaluate(env.clone(), false),
        Instruction::Define(env.clone(), name.clone(), DefineType::Set),
        // Return value is unspecified in the spec.
        Instruction::PushValue(Datum::EmptyList)
    ];
    Ok(instructions)
}

fn special_form_syntax_rules(env: Rc<RefCell<Environment>>, args: &[Datum]) ->
    Result<Vec<Instruction>, RuntimeError>
{
    let usage_str =
        format!("Usage: (syntax-rules (keywords) ((pattern) template) ...)");
    if args.len() < 2 { runtime_error!("{}", &usage_str); }

    // Parse the keywords list.
    let mut keywords = Vec::new();
    for keyword in try!(args[0].to_vec()).iter() {
        keywords.push(match keyword {
            &Datum::Symbol(ref s) => s.clone(),
            _ => runtime_error!("{}", &usage_str)
        });
    }
    if keywords.contains(&String::from("...")) {
        runtime_error!("Ellipses (...) cannot be in the keywords list");
    }

    // Parse the pattern/template entries.
    let mut pattern_templates = Vec::new();
    for pt in args.iter().skip(1) {
        let mut parts = try!(pt.to_vec());
        if parts.len() != 2 { runtime_error!("{}", &usage_str); }
        let template = parts.remove(1);
        let pattern_datum = parts.remove(0);
        let pattern = match pattern_datum {
            Datum::Pair(ref car, ref cdr) => {
                match **car {
                    Datum::Symbol(_) => *cdr.clone(),
                    _ => runtime_error!("First element in a pattern must be the macro identifier")
                }
            }
            _ => runtime_error!("{}", &usage_str)
        };

        // Verify the pattern and template.
        let variables = try!(verify_pattern(&pattern, &keywords));
        let template_symbols = try!(verify_template(&template));

        // Environment to hold any free variables in the template.
        let mut free_env = Environment::new();
        for sym in template_symbols.iter() {
            if !variables.contains(sym) {
                if let Some(val) = env.borrow().get(sym) {
                    free_env.define(sym, val.clone());
                }
            }
        }

        pattern_templates.push((pattern, template, template_symbols, free_env));
    }

    // Create a function that takes in a raw form and attempts to match
    // it against the patterns. If one matches, it applies the associated
    // template and evaluates the result.
    let func = Datum::special(move |env: Rc<RefCell<Environment>>,
        args: &[Datum]|
    {
        // Verify that a raw un-expanded macro call has been passed.
        if args.len() != 1 { runtime_error!("Expected 1 arg"); }
        let (macro_name, input) = match args[0] {
            Datum::Pair(ref car, ref cdr) => {
                match **car {
                    Datum::Symbol(ref s) => (s.clone(), *cdr.clone()),
                    _ => runtime_error!("First element in a pattern must be the macro identifier")
                }
            },
            _ => runtime_error!("Cannot apply syntax-rules to non-list")
        };

        // Try to match against each pattern in order.
        for &(ref pattern, ref template, ref template_syms, ref free_env) in
            pattern_templates.iter()
        {
            // Try to match against this pattern.
            match match_pattern(pattern, &input, &keywords) {
                Some(var_env) => {
                    // === MACRO HYGIENE ===
                    // Rename symbols in the template for hygiene.
                    let mut name_mappings = HashMap::new();
                    for template_sym in template_syms.iter() {
                        // Rename the symbol if it exists in the current
                        // environment so as not to conflict.
                        let mut new_name = template_sym.clone();
                        let mut temp_index = 1;
                        while let Some(_) = env.borrow().get(&new_name) {
                            new_name = format!("{}_hygienic_{}",
                                template_sym, temp_index);
                            temp_index += 1;
                        }
                        name_mappings.insert(template_sym.clone(), new_name);
                    }
                    name_mappings.insert(macro_name.clone(),macro_name.clone());
                    let renamed_template = rename_template(&template,
                        &name_mappings);
                    
                    // The evaluation environment for the template
                    // is the current environment plus the values of
                    // free variables stored when the macro was defined.
                    let eval_env = Rc::new(RefCell::new(
                        Environment::with_parent(env.clone())));
                    for (old_name, new_name) in name_mappings {
                        match free_env.get(&old_name) {
                            Some(d) => eval_env.borrow_mut().
                                define(&new_name, d),
                            None => (),
                        }
                    }

                    // Apply the template.
                    let result = try!(apply_template(&renamed_template,
                        &var_env));
                    return Ok(vec![
                        Instruction::PushValue(result),
                        Instruction::Evaluate(eval_env.clone(), false)
                    ]);
                },
                None => ()
            }
        }
        runtime_error!("Failed to match any syntax patterns");
    });

    Ok(vec![Instruction::PushValue(func)])
}

// Renames symbols in the template according to the given mappings.
fn rename_template(template: &Datum, mappings: &HashMap<String, String>) ->
    Datum
{
    match template {
        &Datum::Symbol(ref s) => {
            match mappings.get(s) {
                Some(m) => Datum::Symbol(m.clone()),
                None => template.clone()
            }
        },
        &Datum::Pair(ref car, ref cdr) =>
            Datum::pair(rename_template(&car, mappings),
                rename_template(&cdr, mappings)),
        _ => template.clone()
    }
}

// Returns the names of all pattern variables if successful.
// Duplicates are not allowed.
fn verify_pattern(pattern: &Datum, keywords: &[String]) ->
    Result<HashSet<String>, RuntimeError>
{
    let mut variables = HashSet::new();
    try!(verify_pattern_helper(pattern, keywords, true, &mut variables));
    Ok(variables)
}

fn verify_pattern_helper(pattern: &Datum, keywords: &[String], list_begin: bool,
    variables: &mut HashSet<String>) -> Result<(), RuntimeError>
{
    match pattern {
        &Datum::Symbol(ref s) if !keywords.contains(s) && s != "..." => {
            if variables.contains(s) {
                runtime_error!("Duplicate pattern variables are not allowed");
            }
            variables.insert(s.clone());
            Ok(())
        },
        &Datum::Pair(ref car, ref cdr) => {
            // Check for ellipses. They should only be found at the
            // end of a list following a pattern.
            match **car {
                Datum::Symbol(ref s) if s == "..." => {
                    let list_end = match **cdr {
                        Datum::EmptyList => true,
                        _ => false
                    };
                    let follows_pattern = !list_begin;
                    if !list_end || !follows_pattern {
                        runtime_error!("Ellipses can only occur at the end of a list and must follow a pattern");
                    }
                },
                _ => ()
            }

            // Recursively verify the elements of the pair.
            try!(verify_pattern_helper(car, keywords, true, variables));
            try!(verify_pattern_helper(cdr, keywords, false, variables));
            Ok(())
        },
        _ => Ok(())
    }
}

// Returns the symbols in the template if successful.
fn verify_template(template: &Datum) -> Result<HashSet<String>, RuntimeError> {
    let mut symbols = HashSet::new();
    try!(verify_template_helper(template, true, &mut symbols));
    Ok(symbols)
}

fn verify_template_helper(template: &Datum, list_begin: bool,
    symbols: &mut HashSet<String>) -> Result<(), RuntimeError>
{
    match template {
        &Datum::Symbol(ref s) if s != "..." => {
            if !symbols.contains(s) {
                symbols.insert(s.clone());
            }
            Ok(())
        },
        &Datum::Pair(ref car, ref cdr) => {
            // Check for ellipses- they should only be following a pattern.
            match **car {
                Datum::Symbol(ref s) if s == "..." => {
                    let follows_pattern = !list_begin;
                    if !follows_pattern {
                        runtime_error!("Ellipses must follow a pattern");
                    }
                },
                _ => ()
            }

            // Recursively verify the elements of the pair.
            try!(verify_template_helper(car, true, symbols));
            try!(verify_template_helper(cdr, false, symbols));
            Ok(())
        },
        _ => Ok(())
    }
}

// Attempts to match the input to the given pattern. If successful,
// an environment of the pattern variables is returned.
fn match_pattern(pattern: &Datum, input: &Datum, keywords: &[String]) ->
    Option<Environment>
{
    let mut env = Environment::new();
    if match_pattern_helper(pattern, input, keywords, &mut env) {
        Some(env)
    } else {
        None
    }
}

fn match_pattern_helper(pattern: &Datum, input: &Datum, keywords: &[String],
    env: &mut Environment) -> bool
{
    match (pattern, input) {
        // Keyword literal.
        (&Datum::Symbol(ref s), inp @ _) if keywords.contains(s) => {
            match inp {
                &Datum::Symbol(ref t) => s == t,
                _ => false
            }
        },
        // Pattern variable.
        (&Datum::Symbol(ref s), inp @ _) => {
            env.define(s, inp.clone());
            true
        },
        // TODO: Implement this.
        (&Datum::Vector(..), _) => unimplemented!(),
        (&Datum::Procedure(..), _) => false,
        (&Datum::SyntaxRule(..), _) => false,
        (&Datum::Pair(ref pcar, ref pcdr), inp @ _) => {
            let zero_or_more = match **pcdr {
                Datum::Pair(ref next, _) => {
                    match **next {
                        Datum::Symbol(ref s) if s == "..." => true,
                        _ => false
                    }
                },
                _ => false
            };
            if zero_or_more {
                // Match as long as possible.
                let mut current = inp;
                let mut at_least_one_found = false;
                let mut to_reverse = HashSet::new();
                loop {
                    // Make sure the current is part of a list.
                    let (element, next) = match current {
                        &Datum::Pair(ref ccar, ref ccdr) => (ccar, ccdr),
                        &Datum::EmptyList => break,
                        // Not a list so doesn't match.
                        _ => return false
                    };

                    // Check if the list element matches the pattern.
                    let mut sub_env = Environment::new();
                    if !match_pattern_helper(pcar, &element, keywords,
                        &mut sub_env)
                    {
                        return false;
                    }

                    // Merge in the sub environment.
                    for (var, value) in sub_env.iter() {
                        let mut curr = if let Some(d) = env.get(var) {
                            d
                        } else {
                            Datum::EmptyList
                        };
                        curr = Datum::pair(value.clone(), curr);
                        env.define(var, curr);
                        to_reverse.insert(var.clone());
                    }

                    // Move to the next element.
                    current = &**next;
                    at_least_one_found = true;
                }

                // If no matches were found, add an empty list for each
                // variable in the pattern.
                if !at_least_one_found {
                    add_empty_matching(pcar, keywords, env);
                }

                // Reverse any lists that were built up.
                for var in to_reverse {
                    let value = env.get(&var).unwrap();
                    env.define(&var, value.reverse());
                }
                true
            } else {
                // Continue matching one at a time.
                match inp {
                    &Datum::Pair(ref icar, ref icdr) => {
                        match_pattern_helper(pcar, icar, keywords, env) &&
                            match_pattern_helper(pcdr, icdr, keywords, env)
                    },
                    _ => false
                }
            }
        },
        (p @ _, inp @ _) => p == inp
    }
}

fn add_empty_matching(pattern: &Datum, keywords: &[String],
    env: &mut Environment)
{
    match pattern {
        &Datum::Symbol(ref s) if !keywords.contains(s) => {
            env.define(s, Datum::EmptyList);
        },
        &Datum::Pair(ref car, ref cdr) => {
            add_empty_matching(car, keywords, env);
            add_empty_matching(cdr, keywords, env);
        },
        _ => ()
    }
}

fn get_variables(template: &Datum, var_env: &Environment) -> HashSet<String> {
    let mut variables = HashSet::new();
    get_variables_helper(template, var_env, &mut variables);
    variables
}

fn get_variables_helper(template: &Datum, var_env: &Environment,
    variables: &mut HashSet<String>)
{
    match template {
        &Datum::Symbol(ref s) if var_env.contains(s) && s != "..." => {
            variables.insert(s.clone());
        },
        &Datum::Pair(ref car, ref cdr) => {
            get_variables_helper(car, var_env, variables);
            get_variables_helper(cdr, var_env, variables);
        },
        _ => ()
    }
}

fn apply_template(template: &Datum, var_env: &Environment) ->
    Result<Datum, RuntimeError>
{
    match template {
        // Handle variable substitution.
        &Datum::Symbol(ref s) if var_env.contains(s) =>
            Ok(var_env.get(s).unwrap()),
        &Datum::Pair(ref car, ref cdr) => {
            let (zero_or_more, after) = match **cdr {
                Datum::Pair(ref next, ref after) => {
                    match **next {
                        Datum::Symbol(ref s) if s == "..." =>
                            (true, Some(after)),
                        _ => (false, None)
                    }
                },
                _ => (false, None)
            };
            if zero_or_more {
                // Determine which variables need to be iterated over for
                // the ellipses.
                let variables = get_variables(car, var_env);
                if variables.len() == 0 {
                    runtime_error!("Expected variables before ellipses");
                }
                let vectors: Vec<(String, Vec<Datum>)> = variables.iter()
                    .map(|v| (v.clone(), var_env.get(v).unwrap().as_vec().0))
                    .collect();
                let iterations = vectors.iter()
                    .map(|v| v.1.len()).min().unwrap();

                // Iterate over variables and build up a list (backwards).
                let mut reversed = Datum::EmptyList;
                for i in 0..iterations {
                    let mut sub_env = Environment::new();
                    for &(ref var, ref values) in vectors.iter() {
                        sub_env.define(&var, values[i].clone());
                    }
                    let result = try!(apply_template(car, &sub_env));
                    reversed = Datum::pair(result, reversed);
                }

                // Recursively apply the template to the rest.
                let mut result = try!(apply_template(after.unwrap(), var_env));

                // Unreverse the list as it is attached to the rest.
                let mut current = &reversed;
                loop {
                    current = match current {
                        &Datum::EmptyList => break,
                        &Datum::Pair(ref a, ref b) => {
                            result = Datum::pair(*a.clone(), result);
                            &*b
                        },
                        _ => panic!("bug in apply_template")
                    };
                }

                Ok(result)
            } else {
                Ok(Datum::pair(try!(apply_template(car, var_env)),
                    try!(apply_template(cdr, var_env))))
            }
        },
        t @ _ => Ok(t.clone())
    }
}

fn native_add(args: &[Datum]) -> Result<Datum, RuntimeError> {
    let mut sum = 0;
    for a in args {
        sum += try_unwrap_arg!(*a => i64);
    }

    Ok(Datum::Number(sum))
}

fn native_subtract(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args >= 1);

    let mut difference = 0;
    for (i, a) in args.iter().enumerate() {
        let n = try_unwrap_arg!(*a => i64);
        difference = if i == 0 { n } else { difference - n };
    }

    // Handle unary case.
    if args.len() == 1 { Ok(Datum::Number(-difference)) }
    else { Ok(Datum::Number(difference)) }
}

fn native_append(args: &[Datum]) -> Result<Datum, RuntimeError> {
    if args.len() == 0 { return Ok(Datum::EmptyList); }
    let mut result = vec![];
    let last_loc = args.len() - 1;
    let last_arg = args[last_loc].clone();
    for arg in &args[0..args.len() - 1] {
        let v = try!(arg.to_vec());
        result.extend(v);
    }
    result.push(last_arg);
    Ok(Datum::improper_list(result))
}

fn native_car(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    match args[0] {
        Datum::Pair(ref car, _) => Ok(*car.clone()),
        _ => runtime_error!("Expected pair")
    }
}

fn native_cdr(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    match args[0] {
        Datum::Pair(_, ref cdr) => Ok(*cdr.clone()),
        _ => runtime_error!("Expected pair")
    }
}

fn native_cons(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    Ok(Datum::Pair(Box::new(args[0].clone()), Box::new(args[1].clone())))
}

fn native_equals(args: &[Datum]) -> Result<Datum, RuntimeError> {
    if args.len() == 0 {
        return Ok(Datum::Boolean(true));
    }

    let first = match args[0] {
        Datum::Number(n) => n,
        _ => runtime_error!("Expected number")
    };

    let mut res = true;
    for a in &args[1..] {
        res = res && (try_unwrap_arg!(*a => i64) == first);
    }

    Ok(Datum::Boolean(res))
}

fn native_multiply(args: &[Datum]) -> Result<Datum, RuntimeError> {
    let mut product = 1;
    for a in args {
        product *= try_unwrap_arg!(*a => i64);
    }

    Ok(Datum::Number(product))
}

fn native_equal_p(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);

    match (&args[0], &args[1]) {
        (&Datum::Boolean(ref b1), &Datum::Boolean(ref b2)) =>
            Ok(Datum::Boolean(b1 == b2)),
        (&Datum::Symbol(ref s1), &Datum::Symbol(ref s2)) =>
            Ok(Datum::Boolean(s1 == s2)),
        (&Datum::Number(ref n1), &Datum::Number(ref n2)) =>
            Ok(Datum::Boolean(n1 == n2)),
        (&Datum::Character(ref c1), &Datum::Character(ref c2)) =>
            Ok(Datum::Boolean(c1 == c2)),
        (&Datum::Ext(ref e1), &Datum::Ext(ref e2)) =>
            Ok(Datum::Boolean(e1 == e2)),
        (&Datum::EmptyList, &Datum::EmptyList) => Ok(Datum::Boolean(true)),
        (&Datum::Procedure(ref p1), &Datum::Procedure(ref p2)) => {
            match (p1, p2) {
                // Note: compare pointers here.
                (&Procedure::SpecialForm(ref s1),
                    &Procedure::SpecialForm(ref s2)) =>
                        Ok(Datum::Boolean(&(**s1) as *const _ ==
                                          &(**s2) as *const _)),
                (&Procedure::Native(ref n1),
                    &Procedure::Native(ref n2)) =>
                        Ok(Datum::Boolean(&(**n1) as *const _ ==
                                          &(**n2) as *const _)),
                (&Procedure::Scheme(ref s1),
                    &Procedure::Scheme(ref s2)) =>
                        Ok(Datum::Boolean(&(**s1) as *const _ ==
                                          &(**s2) as *const _)),
                _ => Ok(Datum::Boolean(false))
            }
        },
        (&Datum::Pair(ref car1, ref cdr1),&Datum::Pair(ref car2, ref cdr2)) => {
            let car_result = try!(
                native_equal_p(&vec![*car1.clone(), *car2.clone()]));
            let cdr_result = try!(
                native_equal_p(&vec![*cdr1.clone(), *cdr2.clone()]));
            Ok(Datum::Boolean(match (car_result, cdr_result) {
                (Datum::Boolean(false), _) => false,
                (_, Datum::Boolean(false)) => false,
                _ => true
            }))
        },
        (&Datum::Vector(ref v1), &Datum::Vector(ref v2)) => {
            if v1.borrow().len() != v2.borrow().len() {
                return Ok(Datum::Boolean(false));
            }
            for (e1, e2) in v1.borrow().iter().zip(v2.borrow().iter()) {
                match try!(native_equal_p(&vec![e1.clone(), e2.clone()])) {
                    d @ Datum::Boolean(false) => return Ok(d),
                    _ => ()
                }
            }
            Ok(Datum::Boolean(true))
        },
        (&Datum::String(ref s1), &Datum::String(ref s2)) =>
            Ok(Datum::Boolean(s1 == s2)),
        _ => Ok(Datum::Boolean(false))
    }
}

fn native_eqv_p(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);

    match (&args[0], &args[1]) {
        (&Datum::Boolean(ref b1), &Datum::Boolean(ref b2)) =>
            Ok(Datum::Boolean(b1 == b2)),
        (&Datum::Symbol(ref s1), &Datum::Symbol(ref s2)) =>
            Ok(Datum::Boolean(s1 == s2)),
        (&Datum::Number(ref n1), &Datum::Number(ref n2)) =>
            Ok(Datum::Boolean(n1 == n2)),
        (&Datum::Character(ref c1), &Datum::Character(ref c2)) =>
            Ok(Datum::Boolean(c1 == c2)),
        (&Datum::EmptyList, &Datum::EmptyList) => Ok(Datum::Boolean(true)),
        (&Datum::Procedure(ref p1), &Datum::Procedure(ref p2)) => {
            match (p1, p2) {
                // Note: compare pointers here.
                (&Procedure::SpecialForm(ref s1),
                    &Procedure::SpecialForm(ref s2)) =>
                        Ok(Datum::Boolean(&(**s1) as *const _ ==
                                          &(**s2) as *const _)),
                (&Procedure::Native(ref n1),
                    &Procedure::Native(ref n2)) =>
                        Ok(Datum::Boolean(&(**n1) as *const _ ==
                                          &(**n2) as *const _)),
                (&Procedure::Scheme(ref s1),
                    &Procedure::Scheme(ref s2)) =>
                        Ok(Datum::Boolean(&(**s1) as *const _ ==
                                          &(**s2) as *const _)),
                _ => Ok(Datum::Boolean(false))
            }
        },
        (&Datum::Vector(ref v1), &Datum::Vector(ref v2)) =>
            Ok(Datum::Boolean(&(**v1) as *const _ ==
                              &(**v2) as *const _)),
        (&Datum::Pair(..), &Datum::Pair(..)) | 
        (&Datum::String(..), &Datum::String(..)) =>
            Ok(Datum::Boolean(false)),
        _ => Ok(Datum::Boolean(false))
    }
}

fn native_hash_ref(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    let h = try_unwrap_arg!(args[0] =>
                            Rc<RefCell<HashMap<Datum, Datum>>>);

    // Make sure the key can be hashed.
    match args[1] {
        // TODO: Support Ext for hashing.
        Datum::Procedure(_) | Datum::SyntaxRule(..) | Datum::Ext(..) =>
            return Ok(Datum::Boolean(false)),
        _ => ()
    }

    match h.borrow().get(&args[1]) {
        Some(d) => Ok(d.clone()),
        None => Ok(Datum::Boolean(false))
    }
}

fn native_hash_set(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 3);
    let h = try_unwrap_arg!(args[0] =>
                            Rc<RefCell<HashMap<Datum, Datum>>>);

    // Make sure the key can be hashed.
    match args[1] {
        // TODO: Support Ext for hashing.
        Datum::Procedure(_) | Datum::SyntaxRule(..) | Datum::Ext(..) =>
            runtime_error!("Hashing not supported for {}", args[1]),
        _ => ()
    }

    h.borrow_mut().insert(args[1].clone(), args[2].clone());
    Ok(args[2].clone())
}

fn native_length(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    Ok(Datum::Number(try!(args[0].to_vec()).len() as i64))
}

fn native_list(args: &[Datum]) -> Result<Datum, RuntimeError> {
    let elements: Vec<_> = args.iter().map(|e| e.clone()).collect();
    Ok(Datum::list(elements))
}

fn native_list_to_string(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    let list = try!(args[0].to_vec());
    let mut string = String::new();
    for d in list {
        let ch = try_unwrap_arg!(d => char);
        string.push(ch);
    }
    Ok(Datum::String(string))
}

fn native_make_hash_table(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 0);
    Ok(Datum::ext(Rc::new(RefCell::new(
        HashMap::<Datum, Datum>::new())), "hash-table"))
}

fn native_null_p(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    match args[0] {
        Datum::EmptyList => Ok(Datum::Boolean(true)),
        _ => Ok(Datum::Boolean(false))
    }
}

fn native_string_append(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    let mut s1 = try_unwrap_arg!(args[0] => String).clone();
    let s2 = try_unwrap_arg!(args[1] => String);
    s1.push_str(s2);
    Ok(Datum::String(s1))
}

fn native_string_contains(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    let s1 = try_unwrap_arg!(args[0] => String).clone();
    let s2 = try_unwrap_arg!(args[1] => String).clone();
    match s1.find(&s2) {
        Some(i) => Ok(Datum::Number(i as i64)),
        None => Ok(Datum::Boolean(false))
    }
}

fn native_reverse(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    match args[0] {
        Datum::EmptyList => (),
        Datum::Pair(..) => (),
        _ => runtime_error!("Expected a list")
    }
    Ok(args[0].reverse())
}

fn native_string_equal_p(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    match (&args[0], &args[1]) {
        (&Datum::String(ref s1), &Datum::String(ref s2)) =>
            Ok(Datum::Boolean(s1 == s2)),
        _ => runtime_error!("Usage: (string=? string1 string2)")
    }
}

fn native_string_length(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    let s = try_unwrap_arg!(args[0] => String);
    Ok(Datum::Number(s.len() as i64))
}

fn native_string_prefix_p(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    let s1 = try_unwrap_arg!(args[0] => String).clone();
    let s2 = try_unwrap_arg!(args[1] => String).clone();
    Ok(Datum::Boolean(s2.starts_with(&s1)))
}

fn native_string_split(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 2);
    let string = try_unwrap_arg!(args[0] => String).clone();
    let ch = try_unwrap_arg!(args[1] => char);
    let splits: Vec<_> = string.split(ch)
        .collect();
    let results = splits.into_iter()
        .map(|s| Datum::String(s.to_string()))
        .collect();
    Ok(Datum::list(results))
}

fn native_string_to_list(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    let s = try_unwrap_arg!(args[0] => String).clone();
    let list: Vec<_> = s.chars().map(|c| Datum::Character(c)).collect();
    Ok(Datum::list(list))
}

fn native_string_to_number(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    let s = try_unwrap_arg!(args[0] => String).clone();
    match s.parse::<i64>() {
        Ok(n) => Ok(Datum::Number(n)),
        Err(_) => runtime_error!("Cannot convert {} to a number", &s)
    }
}

fn native_string_to_symbol(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    let s = try_unwrap_arg!(args[0] => String).clone();
    Ok(Datum::Symbol(s))
}

fn native_substring(args: &[Datum]) -> Result<Datum, RuntimeError> {
    if args.len() != 2 && args.len() != 3 {
        runtime_error!("Usage: (substring str start [end])");
    }
    let string = try_unwrap_arg!(args[0] => String);
    let start = try_unwrap_arg!(args[1] => i64) as usize;
    let end = if args.len() == 3 { try_unwrap_arg!(args[2] => i64) as usize }
        else { string.len() };
    // TODO: Fix i64 <-> usize conversion.
    if end > string.len() || start > string.len() || start > end {
        runtime_error!("Cannot index string from {} to {}", start, end);
    }

    Ok(Datum::String((&string[start..end]).to_string()))
}

fn native_symbol_to_string(args: &[Datum]) -> Result<Datum, RuntimeError> {
    expect_args!(args == 1);
    let s = try_unwrap_arg!(args[0] => Symbol).clone();
    Ok(Datum::String(s))
}

macro_rules! datum_predicate{
    ($dtype:path, $func:ident) => (
        fn $func(args: &[Datum]) -> Result<Datum, RuntimeError> {
            expect_args!(args == 1);

            match args[0] {
                $dtype(..) => Ok(Datum::Boolean(true)),
                _ => Ok(Datum::Boolean(false))
            }
        }
    )
}

datum_predicate!(Datum::Boolean, native_boolean_p);
datum_predicate!(Datum::Character, native_char_p);
datum_predicate!(Datum::Number, native_number_p);
datum_predicate!(Datum::Pair, native_pair_p);
datum_predicate!(Datum::Procedure, native_procedure_p);
datum_predicate!(Datum::String, native_string_p);
datum_predicate!(Datum::Symbol, native_symbol_p);
datum_predicate!(Datum::Vector, native_vector_p);
