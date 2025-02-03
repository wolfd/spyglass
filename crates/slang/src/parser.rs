use anyhow::Result;
use pest::{iterators::Pair, pratt_parser::PrattParser, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "slang.pest"]
struct SlangParser;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Ident(String),
    Call {
        name: String, // TODO(danny): consider Expr here
        args: Vec<Expr>,
    },
    Attribute {
        obj: Box<Expr>,
        attr: String,
    },
    ArrayIndex {
        obj: Box<Expr>,
        index: Box<Expr>,
    },
    ArraySlice {
        obj: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
    },
    BinOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Modulus,
}

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;

        // Precedence is defined lowest to highest
        PrattParser::new()
            // Addition and subtract have equal precedence
            .op(Op::infix(add, Left) | Op::infix(subtract, Left))
            .op(Op::infix(multiply, Left) | Op::infix(divide, Left) | Op::infix(modulus, Left))
            .op(Op::infix(power, Left))
    };
}

fn parse_basic_val(pair: Pair<'_, Rule>) -> Result<Expr> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::int => Ok(Expr::Int(first.as_str().parse::<i64>().unwrap())),
        Rule::float => Ok(Expr::Float(first.as_str().parse::<f64>().unwrap())),
        Rule::ident => {
            let ident = Expr::Ident(first.as_str().to_string());
            let mut val = ident;
            for p in inner {
                val = match p.as_rule() {
                    Rule::call => {
                        let mut args = vec![];
                        for arg in p.into_inner() {
                            args.push(parse_basic_expression(arg)?);
                        }
                        Expr::Call {
                            name: first.as_str().to_string(),
                            args,
                        }
                    }
                    Rule::attribute => {
                        Expr::Attribute {
                            obj: Box::new(val),
                            // Cut off the leading dot
                            attr: p.as_str()[1..].to_string(),
                        }
                    }
                    Rule::slice => {
                        let mut start_or_index = None;
                        let mut has_slice_sep = false;
                        let mut end = None;

                        for part in p.into_inner() {
                            match part.as_rule() {
                                Rule::basic_expr => {
                                    if has_slice_sep {
                                        end = Some(parse_basic_expression(part)?);
                                    } else {
                                        start_or_index = Some(parse_basic_expression(part)?);
                                    }
                                }
                                Rule::slice_sep => {
                                    has_slice_sep = true;
                                }
                                _ => unreachable!(),
                            }
                        }

                        if has_slice_sep {
                            Expr::ArraySlice {
                                obj: Box::new(val),
                                start: start_or_index.map(Box::new),
                                end: end.map(Box::new),
                            }
                        } else {
                            if end.is_some() {
                                unreachable!("can't have two expressions in an array index");
                            }
                            Expr::ArrayIndex {
                                obj: Box::new(val),
                                index: Box::new(start_or_index.unwrap()),
                            }
                        }
                    }
                    rule => unreachable!("parse_basic_val expected trailer, found {:?}", rule),
                };
            }
            Ok(val)
        }

        rule => unreachable!("parse_basic_val expected basic_val, found {:?}", rule),
    }
}

fn parse_basic_expression(pair: Pair<'_, Rule>) -> Result<Expr> {
    let primary = parse_basic_expression;

    let infix = |lhs: Result<Expr>, op: Pair<'_, Rule>, rhs: Result<Expr>| {
        Ok(Expr::BinOp {
            lhs: Box::new(lhs?),
            op: match op.as_rule() {
                Rule::add => Op::Add,
                Rule::subtract => Op::Subtract,
                Rule::multiply => Op::Multiply,
                Rule::divide => Op::Divide,
                Rule::power => Op::Power,
                Rule::modulus => Op::Modulus,
                rule => unreachable!(
                    "parse_basic_expression expected infix operation, found {:?}",
                    rule
                ),
            },
            rhs: Box::new(rhs?),
        })
    };

    let expr = match pair.as_rule() {
        Rule::basic_val => parse_basic_val(pair)?,
        Rule::basic_expr => PRATT_PARSER
            .map_primary(primary)
            .map_infix(infix)
            .parse(pair.into_inner())?,
        rule => unreachable!("parse_basic_expression expected atom, found {:?}", rule),
    };
    Ok(expr)
}

pub fn parse(input: &str) -> Result<Expr> {
    let mut pairs = SlangParser::parse(Rule::calculation, input)?;
    for p in pairs.next().unwrap().into_inner() {
        match p.as_rule() {
            Rule::basic_expr => {
                return parse_basic_expression(p);
            }
            Rule::EOI => return Err(anyhow::anyhow!("incomplete expression")),
            rule => unreachable!("parse expected basic_expr, found {:?}", rule),
        }
    }

    Err(anyhow::anyhow!("no expression found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_math() {
        assert_eq!(
            parse("5 + 5").unwrap(),
            Expr::BinOp {
                lhs: Box::new(Expr::Int(5)),
                op: Op::Add,
                rhs: Box::new(Expr::Int(5)),
            }
        );

        assert_eq!(
            parse("1 + 2 * 3").unwrap(),
            Expr::BinOp {
                lhs: Box::new(Expr::Int(1)),
                op: Op::Add,
                rhs: Box::new(Expr::BinOp {
                    lhs: Box::new(Expr::Int(2)),
                    op: Op::Multiply,
                    rhs: Box::new(Expr::Int(3)),
                }),
            }
        );

        assert_eq!(
            parse("(1 + 2) * 3").unwrap(),
            Expr::BinOp {
                lhs: Box::new(Expr::BinOp {
                    lhs: Box::new(Expr::Int(1)),
                    op: Op::Add,
                    rhs: Box::new(Expr::Int(2)),
                }),
                op: Op::Multiply,
                rhs: Box::new(Expr::Int(3)),
            }
        );

        assert_eq!(
            parse("1 + 2 + 3").unwrap(),
            Expr::BinOp {
                lhs: Box::new(Expr::BinOp {
                    lhs: Box::new(Expr::Int(1)),
                    op: Op::Add,
                    rhs: Box::new(Expr::Int(2)),
                }),
                op: Op::Add,
                rhs: Box::new(Expr::Int(3)),
            }
        );
    }

    #[test]
    fn test_parse_dotted() {
        assert_eq!(parse("utime").unwrap(), Expr::Ident("utime".to_string()));

        assert_eq!(
            parse("utime * 1.0e-6").unwrap(),
            Expr::BinOp {
                lhs: Box::new(Expr::Ident("utime".to_string())),
                op: Op::Multiply,
                rhs: Box::new(Expr::Float(1.0e-6)),
            }
        );

        assert_eq!(
            parse("position.data[0]").unwrap(),
            Expr::ArrayIndex {
                obj: Box::new(Expr::Attribute {
                    obj: Box::new(Expr::Ident("position".to_string())),
                    attr: "data".to_string(),
                }),
                index: Box::new(Expr::Int(0)),
            }
        );
    }

    #[test]
    fn test_parse_index() {
        assert_eq!(
            parse("data[0]").unwrap(),
            Expr::ArrayIndex {
                obj: Box::new(Expr::Ident("data".to_string())),
                index: Box::new(Expr::Int(0)),
            }
        );

        assert_eq!(
            parse("data[other_value + 1]").unwrap(),
            Expr::ArrayIndex {
                obj: Box::new(Expr::Ident("data".to_string())),
                index: Box::new(Expr::BinOp {
                    lhs: Box::new(Expr::Ident("other_value".to_string())),
                    op: Op::Add,
                    rhs: Box::new(Expr::Int(1)),
                }),
            }
        );

        assert_eq!(
            parse("data[:]").unwrap(),
            Expr::ArraySlice {
                obj: Box::new(Expr::Ident("data".to_string())),
                start: None,
                end: None,
            }
        );

        assert_eq!(
            parse("data[10:]").unwrap(),
            Expr::ArraySlice {
                obj: Box::new(Expr::Ident("data".to_string())),
                start: Some(Box::new(Expr::Int(10))),
                end: None,
            }
        );

        assert_eq!(
            parse("data[:10]").unwrap(),
            Expr::ArraySlice {
                obj: Box::new(Expr::Ident("data".to_string())),
                start: None,
                end: Some(Box::new(Expr::Int(10))),
            }
        );

        assert_eq!(
            parse("data[10:20]").unwrap(),
            Expr::ArraySlice {
                obj: Box::new(Expr::Ident("data".to_string())),
                start: Some(Box::new(Expr::Int(10))),
                end: Some(Box::new(Expr::Int(20))),
            }
        );
    }

    #[test]
    fn test_parse_call() {
        assert_eq!(
            parse("func()").unwrap(),
            Expr::Call {
                name: "func".to_string(),
                args: vec![],
            }
        );

        assert_eq!(
            parse("func(1, 2)").unwrap(),
            Expr::Call {
                name: "func".to_string(),
                args: vec![Expr::Int(1), Expr::Int(2)],
            }
        );

        assert_eq!(
            parse("func(1, 2 + 3)").unwrap(),
            Expr::Call {
                name: "func".to_string(),
                args: vec![
                    Expr::Int(1),
                    Expr::BinOp {
                        lhs: Box::new(Expr::Int(2)),
                        op: Op::Add,
                        rhs: Box::new(Expr::Int(3)),
                    }
                ],
            }
        );
    }
}
