#![allow(dead_code)]
use crate::eval::*;
use crate::expr::*;
use serde_json::Value;

pub struct Context {
    context: Value,
}

impl Context {
    fn get_value(&self, name: &str) -> Result<&Value, String> {
        let names = name.split(".").collect::<Vec<&str>>();
        if names.is_empty() {
            return Err(format!("Mapping not exist : {}", name));
        }

        let mut value;
        match self.context.get(names.get(0).unwrap()) {
            Some(v) => {
                value = v;
            }
            _ => {
                return Err(format!("Mapping not exist {:?}", names));
            }
        }

        for n in names.iter().skip(1) {
            match value.get(n) {
                Some(v) => {
                    value = v;
                }
                _ => return Err(format!("Mapping not exist : {} at {}", name, n)),
            }
        }

        Ok(value)
    }
}

pub trait ComputeExpr<'a> {
    fn run(&self, context: &'a mut Context) -> Result<Operand, String>;
}

impl<'a> ComputeExpr<'a> for Eval {
    fn run(&self, context: &'a mut Context) -> Result<Operand, String> {
        let mut operands: Vec<Operand> = Vec::new();

        for op in self.expr.iter().rev() {
            match op {
                Op::Operand(operand) => {
                    operands.push(operand.clone());
                }
                optr => {
                    let num_operands = operator_num_operands(optr);
                    if operands.len() < num_operands {
                        return Err(format!(
                            "Number of operands for {:?} is less than {}",
                            optr, num_operands
                        ));
                    }

                    if num_operands == 1 {
                        let op = operands.pop().unwrap();
                        match optr.compute_unary(context, op) {
                            Ok(operand) => {
                                operands.push(operand);
                            }
                            Err(e) => return Err(e),
                        }
                    } else if num_operands == 2 {
                        let right = operands.pop().unwrap();
                        let left = operands.pop().unwrap();

                        match optr.compute_binary(context, left, right) {
                            Ok(operand) => {
                                operands.push(operand);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }

        if operands.len() != 1 {
            return Err(format!(
                "# of operands after computing is not zero. {:?}",
                operands
            ));
        }

        match operands.pop().unwrap() {
            Operand::Object(obj) => Ok(convert_value_to_operand(context.get_value(&obj.name)?)),
            operand => Ok(operand),
        }
    }
}

pub trait Convert {
    fn is_true(&self) -> bool;
    fn to_str(&self) -> String;
}

impl Convert for Operand {
    fn is_true(&self) -> bool {
        match &self {
            Operand::Literal(l) => !l.is_empty(),
            Operand::Number(n) => *n != 0,
            Operand::Decimal(d) => *d != 0.,
            _ => {
                panic!("Unconvertible object {:?}", &self)
            }
        }
    }

    fn to_str(&self) -> String {
        match &self {
            Operand::Literal(l) => l.to_string(),
            Operand::Number(n) => n.to_string(),
            Operand::Decimal(d) => d.to_string(),
            _ => {
                panic!("Unconvertible object {:?}", &self)
            }
        }
    }
}

trait ComputeOp {
    fn compute_binary<'a>(
        &self,
        context: &'a mut Context,
        left: Operand,
        right: Operand,
    ) -> Result<Operand, String>;

    fn compute_unary<'a>(&self, context: &'a Context, op: Operand) -> Result<Operand, String>;
}

impl ComputeOp for Op {
    fn compute_binary<'a>(
        &self,
        context: &'a mut Context,
        left: Operand,
        right: Operand,
    ) -> Result<Operand, String> {
        match self {
            Op::And => return compute_binary(context, left, right, compute_and),
            Op::Or => return compute_binary(context, left, right, compute_or),
            Op::Equal => return compute_binary(context, left, right, compute_eq),
            Op::NotEqual => return compute_binary(context, left, right, compute_neq),
            Op::Greater => return compute_binary(context, left, right, compute_greater),
            Op::GreaterEq => return compute_binary(context, left, right, compute_greater_eq),
            Op::Less => return compute_binary(context, left, right, compute_less),
            Op::LessEq => return compute_binary(context, left, right, compute_less_eq),
            _ => {}
        }

        panic!("Binary {:?} is not implemented", self);
    }

    fn compute_unary<'a>(&self, context: &'a Context, op: Operand) -> Result<Operand, String> {
        match self {
            Op::Not => return compute_unary(context, op, compute_not),
            _ => {}
        }

        panic!("Unary {:?} is not implemented", self);
    }
}

fn convert_value_to_operand(value: &Value) -> Operand {
    match value {
        Value::Bool(b) => Operand::Number(*b as i64),
        Value::Number(n) => {
            if n.is_i64() {
                Operand::Number(n.as_i64().unwrap())
            } else {
                Operand::Decimal(n.as_f64().unwrap())
            }
        }
        Value::String(s) => Operand::Literal(s.clone()),
        _ => {
            panic!("This should not happen")
        }
    }
}

fn compute_unary<'a, ComputeFunc>(
    context: &'a Context,
    op: Operand,
    compute_func: ComputeFunc,
) -> Result<Operand, String>
where
    ComputeFunc: Fn(Operand) -> Result<Operand, String>,
{
    if let Operand::Object(obj) = op {
        match context.get_value(&obj.name) {
            Ok(v) => {
                return compute_unary(context, convert_value_to_operand(v), compute_func);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    compute_func(op)
}

fn compute_binary<'a, ComputeFunc>(
    context: &'a Context,
    left: Operand,
    right: Operand,
    compute_func: ComputeFunc,
) -> Result<Operand, String>
where
    ComputeFunc: Fn(Operand, Operand) -> Result<Operand, String>,
{
    if let Operand::Object(l) = left {
        match context.get_value(&l.name) {
            Ok(v) => {
                return compute_binary(context, convert_value_to_operand(v), right, compute_func);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    if let Operand::Object(r) = right {
        match context.get_value(&r.name) {
            Ok(v) => {
                return compute_func(left, convert_value_to_operand(v));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    compute_func(left, right)
}

fn compute_and<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => {
            Ok(Operand::Number(((l.len() != 0) && (r.len() != 0)) as i64))
        }
        (Operand::Number(l), Operand::Number(r)) => {
            Ok(Operand::Number(((l != &0) && (r != &0)) as i64))
        }
        (Operand::Decimal(l), Operand::Decimal(r)) => {
            Ok(Operand::Number(((l != &0.) && (r != &0.)) as i64))
        }
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_or<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => {
            Ok(Operand::Number(((l.len() != 0) || (r.len() != 0)) as i64))
        }
        (Operand::Number(l), Operand::Number(r)) => {
            Ok(Operand::Number(((l != &0) || (r != &0)) as i64))
        }
        (Operand::Decimal(l), Operand::Decimal(r)) => {
            Ok(Operand::Number(((l != &0.) || (r != &0.)) as i64))
        }
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_greater<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => Ok(Operand::Number((l > r) as i64)),
        (Operand::Number(l), Operand::Number(r)) => Ok(Operand::Number((l > r) as i64)),
        (Operand::Decimal(l), Operand::Decimal(r)) => Ok(Operand::Number((l > r) as i64)),
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_greater_eq<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => Ok(Operand::Number((l >= r) as i64)),
        (Operand::Number(l), Operand::Number(r)) => Ok(Operand::Number((l >= r) as i64)),
        (Operand::Decimal(l), Operand::Decimal(r)) => Ok(Operand::Number((l >= r) as i64)),
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_less<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => Ok(Operand::Number((l < r) as i64)),
        (Operand::Number(l), Operand::Number(r)) => Ok(Operand::Number((l < r) as i64)),
        (Operand::Decimal(l), Operand::Decimal(r)) => Ok(Operand::Number((l < r) as i64)),
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_less_eq<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => Ok(Operand::Number((l <= r) as i64)),
        (Operand::Number(l), Operand::Number(r)) => Ok(Operand::Number((l <= r) as i64)),
        (Operand::Decimal(l), Operand::Decimal(r)) => Ok(Operand::Number((l <= r) as i64)),
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_eq<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => Ok(Operand::Number((l == r) as i64)),
        (Operand::Number(l), Operand::Number(r)) => Ok(Operand::Number((l == r) as i64)),
        (Operand::Decimal(l), Operand::Decimal(r)) => Ok(Operand::Number((l == r) as i64)),
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_neq<'a>(left: Operand, right: Operand) -> Result<Operand, String> {
    match (&left, &right) {
        (Operand::Literal(l), Operand::Literal(r)) => Ok(Operand::Number((l != r) as i64)),
        (Operand::Number(l), Operand::Number(r)) => Ok(Operand::Number((l != r) as i64)),
        (Operand::Decimal(l), Operand::Decimal(r)) => Ok(Operand::Number((l != r) as i64)),
        _ => Err(format!("Type mismatch : {:?} {:?}", left, right)),
    }
}

fn compute_not<'a>(operand: Operand) -> Result<Operand, String> {
    match &operand {
        Operand::Literal(s) => Ok(Operand::Number((s.len() == 0) as i64)),
        Operand::Number(n) => Ok(Operand::Number((n == &0) as i64)),
        Operand::Decimal(d) => Ok(Operand::Number((d == &0.) as i64)),
        _ => Err(format!("Invalid operation NOT {:?}", operand)),
    }
}

#[cfg(test)]
fn get_expr<'a>(s: &'a str) -> Expr {
    let mut res = Parser::parse(s).unwrap();
    match res.parse_tree.pop().unwrap() {
        Action::Do(expr) => expr,
        _ => panic!("No expr found"),
    }
}

#[test]
fn compute_and_test() {
    let context_json = r#"{"a": 1, "b":0, "c":"abc", "d":"", "e": "def"}"#;
    let context_value: Value = serde_json::from_str(context_json).unwrap();
    let mut context = Context {
        context: context_value,
    };

    {
        let eval = Eval::new(get_expr(r"<% a && b %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(0));
    }
    {
        let eval = Eval::new(get_expr(r"<% c && d %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(0));
    }
    {
        let eval = Eval::new(get_expr(r"<% c && e %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(1));
    }
}

#[test]
fn compute_or_test() {
    let context_json = r#"{"a": 1, "b":0, "c":"abc", "d":"", "e": "def"}"#;
    let context_value: Value = serde_json::from_str(context_json).unwrap();
    let mut context = Context {
        context: context_value,
    };

    {
        let eval = Eval::new(get_expr(r"<% (a && b) || (c && e) %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(1));
    }
    {
        let eval = Eval::new(get_expr(r"<% (a && b) || (c && d) %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(0));
    }
    {
        let eval = Eval::new(get_expr(r"<% c || e %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(1));
    }
}

#[test]
fn compute_complex() {
    let context_json = r#"{"a": 1, "b":0, "c":"abc", "d":"", "e": "def"}"#;
    let context_value: Value = serde_json::from_str(context_json).unwrap();
    let mut context = Context {
        context: context_value,
    };

    {
        let eval = Eval::new(get_expr(r"<% !a && ((b != a) || c <= e) %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(0));
    }
    {
        let eval = Eval::new(get_expr(r"<% !b && ((b != a) || c <= e && !d) %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(1));
    }
    {
        let eval = Eval::new(get_expr(
            r#"<% (a == 1) && (b == 0) && (c == "abc") && !d && e == "def" %>"#,
        ))
        .unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(1));
    }
}

#[test]
fn compute_complex_object_name() {
    let context_json = r#"{"a": {"b" : 2, "c" : {"d" : 3 }}, "b" : 1}"#;
    let context_value: Value = serde_json::from_str(context_json).unwrap();
    let mut context = Context {
        context: context_value,
    };

    {
        let eval = Eval::new(get_expr(r"<% a.b %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(2));
    }
    {
        let eval = Eval::new(get_expr(r"<% a.c.d %>")).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(3));
    }
    {
        let eval = Eval::new(get_expr(r#"<% b %>"#)).unwrap();
        let result = eval.run(&mut context);

        assert_eq!(result.unwrap(), Operand::Number(1));
    }
}
