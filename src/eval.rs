#![allow(dead_code)]

use crate::expr::*;

// Evaluate the parsed expression.
#[derive(PartialEq, Debug)]
struct Eval<'a> {
    expr: Vec<Op<'a>>,
}

impl<'a> Eval<'a> {
    pub fn new(mut expr: Expr<'a>) -> Result<Eval<'a>, String> {
        let mut operands: Vec<Vec<Op>> = Vec::new();
        let mut operators = Vec::new();

        // Wrap the expression with ().
        expr.ops.push(Op::ParenClose);
        expr.ops.insert(0, Op::ParenOpen);

        while !expr.ops.is_empty() {
            let op = expr.ops.remove(0);
            match op {
                Op::Operand(operand) => {
                    operands.push(vec![Op::Operand(operand)]);
                }
                optr => {
                    if optr == Op::ParenOpen {
                        operators.push(optr);
                        continue;
                    }

                    if operators.is_empty() {
                        operators.push(optr);
                        continue;
                    }

                    loop {
                        let last_op = operators.last().unwrap();
                        if is_second_priority_higher(last_op, &optr) {
                            operators.push(optr);
                            break;
                        } else if last_op == &Op::ParenOpen && optr == Op::ParenClose {
                            operators.pop();
                            break;
                        } else if operator_num_operands(&optr) == 1
                            && is_second_priority_higher_or_equal(last_op, &optr)
                        {
                            operators.push(optr);
                            break;
                        }

                        let top_optr = operators.pop().unwrap();

                        let mut popped_operands: Vec<Vec<Op>> = Vec::new();
                        for _ in 0..operator_num_operands(&top_optr) {
                            match operands.pop() {
                                Some(operand) => {
                                    popped_operands.push(operand);
                                }
                                _ => {
                                    break;
                                }
                            }
                        }

                        popped_operands.push(vec![top_optr]);
                        popped_operands.reverse();

                        operands.push(popped_operands.into_iter().flatten().collect());
                    }
                }
            }
        }

        Ok(Eval {
            expr: operands.into_iter().flatten().collect(),
        })
    }
}

fn is_second_priority_higher(op1: &Op, op2: &Op) -> bool {
    return operator_priority(op1) > operator_priority(op2);
}

fn is_second_priority_higher_or_equal(op1: &Op, op2: &Op) -> bool {
    return operator_priority(op1) >= operator_priority(op2);
}

fn is_only_unary(operands: &Vec<Op>) -> bool {
    if operands.len() != 1 {
        return false;
    }

    return operator_num_operands(&operands[0]) == 1;
}

#[test]
fn create_simple_unary() {
    let expr = Expr {
        ops: vec![
            Op::Not,
            Op::Operand(Operand::Object(Object { name: "some_value" })),
        ],
    };

    let expected_eval = Eval {
        expr: vec![
            Op::Not,
            Op::Operand(Operand::Object(Object { name: "some_value" })),
        ],
    };

    assert_eq!(Eval::new(expr).unwrap(), expected_eval);
}

#[test]
fn create_multiple_unary_expr() {
    let expr = Expr {
        ops: vec![
            Op::Not,
            Op::Not,
            Op::Not,
            Op::Operand(Operand::Object(Object { name: "some_value" })),
        ],
    };

    let expected_eval = Eval {
        expr: vec![
            Op::Not,
            Op::Not,
            Op::Not,
            Op::Operand(Operand::Object(Object { name: "some_value" })),
        ],
    };
    assert_eq!(Eval::new(expr).unwrap(), expected_eval);
}

#[test]
fn create_simple_binary_expr() {
    let expr = Expr {
        ops: vec![
            Op::Operand(Operand::Object(Object { name: "some" })),
            Op::Equal,
            Op::Operand(Operand::Number(3)),
        ],
    };

    let expected_eval = Eval {
        expr: vec![
            Op::Equal,
            Op::Operand(Operand::Object(Object { name: "some" })),
            Op::Operand(Operand::Number(3)),
        ],
    };
    assert_eq!(Eval::new(expr).unwrap(), expected_eval);
}

#[test]
fn create_binary_with_unary_expr() {
    let expr = Expr {
        ops: vec![
            Op::Operand(Operand::Object(Object { name: "some" })),
            Op::Equal,
            Op::Not,
            Op::Not,
            Op::Operand(Operand::Number(3)),
        ],
    };

    let expected_eval = Eval {
        expr: vec![
            Op::Equal,
            Op::Operand(Operand::Object(Object { name: "some" })),
            Op::Not,
            Op::Not,
            Op::Operand(Operand::Number(3)),
        ],
    };
    assert_eq!(Eval::new(expr).unwrap(), expected_eval);
}

#[test]
fn create_complex_expr_expr() {
    let expr = Expr {
        ops: vec![
            Op::Not,
            Op::Operand(Operand::Object(Object { name: "some_value" })),
            Op::And,
            Op::ParenOpen,
            Op::Not,
            Op::ParenOpen,
            Op::Operand(Operand::Object(Object { name: "var1" })),
            Op::NotEqual,
            Op::Operand(Operand::Object(Object { name: "var2" })),
            Op::ParenClose,
            Op::Or,
            Op::Operand(Operand::Object(Object { name: "some" })),
            Op::LessEq,
            Op::Operand(Operand::Object(Object { name: "val" })),
            Op::ParenClose,
        ],
    };

    let expected_eval = Eval {
        expr: vec![
            Op::And,
            Op::Not,
            Op::Operand(Operand::Object(Object { name: "some_value" })),
            Op::Or,
            Op::Not,
            Op::NotEqual,
            Op::Operand(Operand::Object(Object { name: "var1" })),
            Op::Operand(Operand::Object(Object { name: "var2" })),
            Op::LessEq,
            Op::Operand(Operand::Object(Object { name: "some" })),
            Op::Operand(Operand::Object(Object { name: "val" })),
        ],
    };
    assert_eq!(Eval::new(expr).unwrap(), expected_eval);
}
