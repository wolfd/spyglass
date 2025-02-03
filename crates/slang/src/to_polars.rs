use anyhow::Result;
use polars_lazy::prelude::*;
use std::{collections::HashMap, f64::consts::PI};
use trigonometry::TrigonometricFunction;

use super::parser::{Expr, Op};

// TODO(danny): this
lazy_static::lazy_static! {
    static ref BUILTINS: HashMap<&'static str, FunctionExpr> = {
        let mut m = HashMap::new();
        m.insert("sin", FunctionExpr::Trigonometry(TrigonometricFunction::Sin));
        m.insert("cos", FunctionExpr::Trigonometry(TrigonometricFunction::Cos));
        m.insert("tan", FunctionExpr::Trigonometry(TrigonometricFunction::Tan));
        m.insert("atan2", FunctionExpr::Atan2);
        // m.insert("explode", polars_lazy::dsl::Expr::Explode);
        // ("sin", polars_lazy::dsl::Expr::sin),
        // ("cos", polars_lazy::dsl::Expr::cos),

        m
    };
}

impl TryInto<polars_lazy::dsl::Expr> for Expr {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<polars_lazy::dsl::Expr, Self::Error> {
        to_polars_expr(&self)
    }
}

pub fn to_polars_expr(expr: &Expr) -> Result<polars_lazy::dsl::Expr> {
    match expr {
        Expr::Int(i) => Ok(lit(*i)),
        Expr::Float(f) => Ok(lit(*f)),
        Expr::BinOp { lhs, op, rhs } => {
            let lhs = to_polars_expr(lhs)?;
            let rhs = to_polars_expr(rhs)?;

            Ok(match op {
                Op::Add => lhs + rhs,
                Op::Subtract => lhs - rhs,
                Op::Multiply => lhs * rhs,
                Op::Divide => lhs / rhs,
                Op::Power => lhs.pow(rhs),
                Op::Modulus => lhs % rhs,
            })
        }
        Expr::Ident(name) => Ok(col(name)),
        Expr::Call { name, args } => match name.as_str() {
            // TODO(danny): check for args length
            "explode" => {
                let obj = to_polars_expr(args.into_iter().next().unwrap())?;
                return Ok(obj.explode());
            }
            "sin" => {
                let obj = to_polars_expr(args.into_iter().next().unwrap())?;
                // let thing = BUILTINS.get("sin").unwrap();
                // return obj.map_private(thing);
                return Ok(obj.sin());
            }
            "cos" => {
                let obj = to_polars_expr(args.into_iter().next().unwrap())?;
                return Ok(obj.cos());
            }
            "atan2" => {
                let a = to_polars_expr(args.into_iter().next().unwrap())?;
                let b = to_polars_expr(args.into_iter().next().unwrap())?;
                Ok(polars_lazy::dsl::Expr::arctan2(a, b))
            }
            "roll" => {
                let w = to_polars_expr(args.into_iter().next().unwrap())?;
                let x = to_polars_expr(args.into_iter().next().unwrap())?;
                let y = to_polars_expr(args.into_iter().next().unwrap())?;
                let z = to_polars_expr(args.into_iter().next().unwrap())?;

                // roll (x-axis rotation)
                let sinr_cosp = lit(2) * (w.clone() * x.clone() + y.clone() * z.clone());
                let cosr_cosp = lit(1) - lit(2) * (x.clone().pow(2.0) + y.clone().pow(2.0));
                Ok(polars_lazy::dsl::Expr::arctan2(sinr_cosp, cosr_cosp))
            }
            "pitch" => {
                let w = to_polars_expr(args.into_iter().next().unwrap())?;
                let x = to_polars_expr(args.into_iter().next().unwrap())?;
                let y = to_polars_expr(args.into_iter().next().unwrap())?;
                let z = to_polars_expr(args.into_iter().next().unwrap())?;
                // pitch (y-axis rotation)
                let sinp = polars_lazy::dsl::Expr::sqrt(
                    lit(1) + lit(2) * (w.clone() * y.clone() - x.clone() * z.clone()),
                );
                let cosp = polars_lazy::dsl::Expr::sqrt(
                    lit(1) - lit(2) * (w.clone() * y.clone() - x.clone() * z.clone()),
                );

                Ok(lit(2) * polars_lazy::dsl::Expr::arctan2(sinp, cosp) - lit(PI) / lit(2.0))
            }
            "yaw" => {
                let w = to_polars_expr(args.into_iter().next().unwrap())?;
                let x = to_polars_expr(args.into_iter().next().unwrap())?;
                let y = to_polars_expr(args.into_iter().next().unwrap())?;
                let z = to_polars_expr(args.into_iter().next().unwrap())?;

                // yaw (z-axis rotation)
                let siny_cosp = lit(2) * (w.clone() * z.clone() + x.clone() * y.clone());
                let cosy_cosp = lit(1) - lit(2) * (y.pow(2.0) + z.pow(2.0));
                Ok(polars_lazy::dsl::Expr::arctan2(siny_cosp, cosy_cosp))
            }
            _ => {
                todo!("Call to {} not implemented yet", name);
            }
        },
        Expr::Attribute { obj, attr } => {
            let obj = to_polars_expr(obj)?;

            Ok(obj.struct_().field_by_name(&attr))
        }
        Expr::ArrayIndex { obj, index } => {
            let obj = to_polars_expr(obj)?;
            let index = to_polars_expr(index)?;
            // TODO(danny): Add support for negative indexing
            Ok(obj.list().get(index, true))
        }
        Expr::ArraySlice { obj, start, end } => {
            let obj = to_polars_expr(obj)?;
            let start = if let Some(start) = start {
                Some(to_polars_expr(start)?)
            } else {
                None
            };
            let end = if let Some(end) = end {
                Some(to_polars_expr(end)?)
            } else {
                None
            };

            if let (None, None) = (start, end) {
                Ok(obj)
            } else {
                // TODO(danny): Also do negative support like python
                todo!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_math() {
        let expr = Expr::BinOp {
            lhs: Box::new(Expr::Int(1)),
            op: Op::Add,
            rhs: Box::new(Expr::Int(2)),
        };

        assert_eq!(to_polars_expr(&expr).unwrap(), lit(1) + lit(2));
    }

    #[test]
    fn test_batch() {
        let expr = Expr::Attribute {
            obj: Box::new(Expr::Ident("structs".to_owned())),
            attr: "x".to_owned(),
        };

        assert_eq!(
            to_polars_expr(&expr).unwrap(),
            col("structs").struct_().field_by_name("x")
        );
    }

    #[test]
    fn test_array_index() {
        let expr = Expr::ArrayIndex {
            obj: Box::new(Expr::Ident("lists".to_owned())),
            index: Box::new(Expr::Int(2)),
        };

        assert_eq!(
            to_polars_expr(&expr).unwrap(),
            col("lists").list().get(lit(2), true)
        );
    }

    #[test]
    fn test_call() {
        let expr = Expr::Call {
            name: "explode".to_owned(),
            args: vec![Expr::Ident("lists".to_owned())],
        };

        assert_eq!(to_polars_expr(&expr).unwrap(), col("lists").explode());
    }
}
