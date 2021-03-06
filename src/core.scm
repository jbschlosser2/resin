;; Flow control forms.
(define-syntax and
  (syntax-rules ()
    ((and) #t)
    ((and test) test)
    ((and test1 test2 ...)
     (if test1 (and test2 ...) #f))))

(define-syntax or
  (syntax-rules ()
    ((or) #f)
    ((or test) test)
    ((or test1 test2 ...)
     (let ((x test1))
       (if x x (or test2 ...))))))

(define-syntax cond
  (syntax-rules (else =>)
    ((cond (else result1 result2 ...))
     (begin result1 result2 ...))
    ((cond (test => result))
     (let ((temp test))
       (if temp (result temp))))
    ((cond (test => result) clause1 clause2 ...)
     (let ((temp test))
       (if temp
         (result temp)
         (cond clause1 clause2 ...))))
    ((cond (test)) test)
    ((cond (test) clause1 clause2 ...)
     (let ((temp test))
       (if temp
         temp
         (cond clause1 clause2 ...))))
    ((cond (test result1 result2 ...))
     (if test (begin result1 result2 ...)))
    ((cond (test result1 result2 ...)
           clause1 clause2 ...)
     (if test
       (begin result1 result2 ...)
       (cond clause1 clause2 ...)))))

(define-syntax let*
  (syntax-rules ()
    ((let* () body1 body2 ...)
     (let () body1 body2 ...))
    ((let* ((name1 val1) (name2 val2) ...)
       body1 body2 ...)
     (let ((name1 val1))
       (let* ((name2 val2) ...)
         body1 body2 ...)))))

(define-syntax let
  (syntax-rules ()
    ((let ((name val) ...) body1 body2 ...)
     ((lambda (name ...) body1 body2 ...)
      val ...))
    ((let tag ((name val) ...) body1 body2 ...)
     ((letrec ((tag (lambda (name ...)
                      body1 body2 ...)))
        tag)
      val ...))))

(define-syntax do
  (syntax-rules ()
    ((do ((var init step ...) ...)
       (test expr ...)
       command ...)
     (letrec
       ((loop
          (lambda (var ...)
            (if test
              (begin
                (if #f #f)
                expr ...)
              (begin
                command
                ...
                (loop (do "step" var step ...)
                      ...))))))
       (loop init ...)))
    ((do "step" x)
     x)
    ((do "step" x y)
     y)))

(define apply
  (lambda (proc first . rest)
    (if (null? rest)
        (let loop ((lst (list proc)) (b first))
          (if (null? b)
              (eval lst)
              (loop (append lst (list (list 'quote (car b)))) (cdr b))))
        (let loop ((a (list first)) (b rest))
          (if (= 1 (length b))
              (apply proc (append a (car b)))
              (loop (append a (list (car b))) (cdr b)))))))

(define single-map
  (lambda (proc lst)
    (if (null? lst)
        '()
        (cons (proc (car lst))
              (single-map proc (cdr lst))))))

(define map
  (lambda (proc . lists)
    (if (null? (car lists))
        '()
        (cons (apply proc (single-map car lists))
              (apply map proc (single-map cdr lists))))))
