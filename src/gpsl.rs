use crate::node::*;
use crate::parser::*;
use crate::source::Source;
use crate::tokenizer::*;
use crate::variable::*;
use std::collections::HashMap;
use std::string::*;
use uuid::Uuid;

#[derive(PartialEq)]
pub enum ExternalFuncStatus {
    SUCCESS,
    NOTFOUND,
    ERROR,
}

pub struct ExternalFuncReturn {
    pub status: ExternalFuncStatus,
    pub value: Option<Variable>
}

pub struct GPSL {
    pub functions: Option<Vec<Box<Node>>>,
    pub global_variables: Vec<Variable>,
    pub source: Source,
    pub l_vars: HashMap<String, LocalVariable>,
    pub external_func: Vec<fn(String, Vec<Variable>) -> ExternalFuncReturn>
}

pub struct LocalVariable {
    pub name: String,
    pub value: Variable,
    pub status: VariableStatus,
}

pub struct VariableStatus {
    pub initialized: bool,
}

impl VariableStatus {
    pub fn default() -> VariableStatus {
        VariableStatus { initialized: false }
    }
}

impl GPSL {
    pub fn new(source: Source, functions: Option<Vec<Box<Node>>>, external_func: Vec<fn(String, Vec<Variable>) -> ExternalFuncReturn>) -> GPSL {
        GPSL {
            source,
            functions,
            global_variables: vec![],
            l_vars: HashMap::new(),
            external_func
        }
    }

    pub fn extract_number(node: Variable) -> Result<usize, String> {
        match node {
            Variable::Number { value } => {
                Ok(value)
            },
            _ => {
                Err(String::from("Not a number"))
            }
        }
    }

    pub fn evaluate(&mut self, node: Box<Node>) -> Result<Option<Variable>, String> {
        match *node {
            Node::Call { name, args } => {
                let function_name = name;
                let f = self.external_func.clone();
                let mut args_value: Vec<Variable> = vec![];
                for arg in args {
                    if let Some(val) = self.evaluate(arg).expect("Cannot evaluate") {
                        args_value.push(val);
                    }
                }

                if let Some(functions) = self.functions.clone() {
                    for function in functions {
                        match *function {
                            Node::Function { name, body, args } => {
                                if name == function_name {
                                    for program in body {
                                        if let Ok(Some(res)) = self.evaluate(program) {
                                            match res {
                                                Variable::Return { value } => {
                                                    return Ok(Some(*value));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    return Ok(None);
                                }
                            },
                            _ => {}
                        }
                    }
                }

                for func in f {
                    let res = func(function_name.clone(), args_value.clone());
                    if res.status == ExternalFuncStatus::SUCCESS {
                        return Ok(res.value);
                    }
                }

                Err(format!("Function not found: {}", function_name))
            }
            Node::Text { value } => {
                Ok(Some(Variable::Text {
                    value
                }))
            }
            Node::Number { value } => {
                Ok(Some(Variable::Number {
                    value
                }))
            }
            Node::Operator { kind, lhs, rhs } => {
                if kind == NodeKind::ASSIGN {
                    let rhs = self.evaluate(rhs);

                    if let Ok(Some(rhs)) = rhs {
                        match *(lhs.clone()) {
                            Node::Lvar { value } => {
                                self.l_vars.get_mut(&value).unwrap().value = rhs;
                                self.l_vars.get_mut(&value).unwrap().status.initialized = true
                            }
                            _ => {}
                        }
                    }

                    return Ok(None);
                }
                let lhs = self.evaluate(lhs).expect("Cannot evaluate lhs.");
                let rhs = self.evaluate(rhs).expect("Cannot evaluate rhs.");

                if let Some(lhs) = lhs {
                    if let Some(rhs) = rhs {
                        match kind {
                            NodeKind::ADD => {
                                match GPSL::extract_number(lhs) {
                                    Ok(lhs) => {
                                        match GPSL::extract_number(rhs) {
                                            Ok(rhs) => {
                                                Ok(Some(Variable::Number {
                                                    value: lhs + rhs
                                                }))
                                            }
                                            Err(err) => { Err(err) }
                                        }
                                    }
                                    Err(err) => { Err(err) }
                                }
                            },
                            NodeKind::DIV => {
                                match GPSL::extract_number(lhs) {
                                    Ok(lhs) => {
                                        match GPSL::extract_number(rhs) {
                                            Ok(rhs) => {
                                                Ok(Some(Variable::Number {
                                                    value: lhs / rhs
                                                }))
                                            }
                                            Err(err) => { Err(err) }
                                        }
                                    }
                                    Err(err) => { Err(err) }
                                }
                            },
                            NodeKind::MUL => {
                                match GPSL::extract_number(lhs) {
                                    Ok(lhs) => {
                                        match GPSL::extract_number(rhs) {
                                            Ok(rhs) => {
                                                Ok(Some(Variable::Number {
                                                    value: lhs * rhs
                                                }))
                                            }
                                            Err(err) => { Err(err) }
                                        }
                                    }
                                    Err(err) => { Err(err) }
                                }
                            },
                            NodeKind::SUB => {
                                match GPSL::extract_number(lhs) {
                                    Ok(lhs) => {
                                        match GPSL::extract_number(rhs) {
                                            Ok(rhs) => {
                                                Ok(Some(Variable::Number {
                                                    value: lhs - rhs
                                                }))
                                            }
                                            Err(err) => { Err(err) }
                                        }
                                    }
                                    Err(err) => { Err(err) }
                                }
                            },
        
                            NodeKind::EQ => {
                                if lhs == rhs {
                                    Ok(Some(Variable::Number {
                                        value: 1
                                    }))
                                } else {
                                    Ok(Some(Variable::Number {
                                        value: 0
                                    }))
                                }
                            },
                            NodeKind::NE => {
                                if lhs != rhs {
                                    Ok(Some(Variable::Number {
                                        value: 1
                                    }))
                                } else {
                                    Ok(Some(Variable::Number {
                                        value: 0
                                    }))
                                }
                            },
                            NodeKind::LT => {
                                match GPSL::extract_number(lhs) {
                                    Ok(lhs) => {
                                        match GPSL::extract_number(rhs) {
                                            Ok(rhs) => {
                                                if lhs < rhs {
                                                    Ok(Some(Variable::Number {
                                                        value: 1
                                                    }))
                                                } else {
                                                    Ok(Some(Variable::Number {
                                                        value: 0
                                                    }))
                                                }
                                            }
                                            Err(err) => { Err(err) }
                                        }
                                    }
                                    Err(err) => { Err(err) }
                                }
                            },
                            NodeKind::LE => {
                                match GPSL::extract_number(lhs) {
                                    Ok(lhs) => {
                                        match GPSL::extract_number(rhs) {
                                            Ok(rhs) => {
                                                if lhs <= rhs {
                                                    Ok(Some(Variable::Number {
                                                        value: 1
                                                    }))
                                                } else {
                                                    Ok(Some(Variable::Number {
                                                        value: 0
                                                    }))
                                                }
                                            }
                                            Err(err) => { Err(err) }
                                        }
                                    }
                                    Err(err) => { Err(err) }
                                }
                            },
                            _ => Ok(None)
                        }
                    } else {
                        Err(String::from("RHS Variable is null."))
                    }
                } else {
                    Err(String::from("LHS Variable is null."))
                }
            }
            Node::Lvar { value } => {
                return Ok(Some(self.l_vars.get(&value).unwrap().value.clone()));
            }
            Node::Return { lhs } => {
                if let Ok(Some(lhs)) = self.evaluate(lhs) {
                    return Ok(Some(Variable::Return {
                        value: Box::new(lhs)
                    }));
                } else {
                    return Err(String::from("Cannot evaluate LHS."));
                }
            }
            Node::If {
                condition,
                stmt,
                else_stmt,
            } => {
                if let Ok(Some(condition)) = self.evaluate(condition) {
                    if match condition {
                        Variable::Number { value } => value == 1,
                        _ => false
                    } {
                        if let Ok(Some(res)) = self.evaluate(stmt) {
                            match res.clone() {
                                Variable::Return { value } => {
                                    return Ok(Some(res));
                                }
                                _ => {}
                            }
                        }
                    } else {
                        match else_stmt {
                            Some(else_stmt) => {
                                if let Ok(Some(res)) = self.evaluate(else_stmt) {
                                    match res.clone() {
                                        Variable::Return { value } => {
                                            return Ok(Some(res));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                }

                return Ok(None);
            }
            Node::While { condition, stmt } => {
                let mut cond = if let Some(condition) = self.evaluate(condition.clone())? {
                    condition
                } else {
                    Variable::Number {
                        value: 0
                    }
                };
                
                while match cond {
                    Variable::Number { value } => value == 1,
                    _ => false
                } {
                    self.evaluate(stmt.clone())?;
                    cond = if let Some(condition) = self.evaluate(condition.clone())? {
                        condition
                    } else {
                        Variable::Number {
                            value: 0
                        }
                    };
                }

                return Ok(None);
            }
            Node::For {
                init,
                condition,
                update,
                stmt,
            } => {
                match init {
                    Some(init) => {self.evaluate(init)?;},
                    None => {}
                }

                let mut cond = match condition.clone() {
                    Some(condition) => {
                        if let Some(condition) = self.evaluate(condition)? {
                            condition
                        } else {
                            Variable::Number {
                                value: 0
                            }
                        }
                    },
                    None => { 
                        Variable::Number {
                            value: 1
                        } 
                    }
                };
                
                while match cond {
                    Variable::Number { value } => value == 1,
                    _ => false
                } {
                    self.evaluate(stmt.clone())?;

                    match update.clone() {
                        Some(update) => {self.evaluate(update)?;},
                        None => {}
                    }

                    cond = match condition.clone() {
                        Some(condition) => {
                            if let Some(condition) = self.evaluate(condition)? {
                                condition
                            } else {
                                Variable::Number {
                                    value: 0
                                }
                            }
                        },
                        None => { 
                            Variable::Number {
                                value: 1
                            } 
                        }
                    };
                }

                return Ok(None);
            }
            Node::Block { stmts } => {
                for stmt in stmts {
                    let ret = self.evaluate(stmt)?;
                    if let Some(ret) = ret {
                        match ret.clone() {
                            Variable::Return { value } => {
                                return Ok(Some(ret));
                            }
                            _ => {}
                        }
                    }
                }
                return Ok(None);
            }
            Node::Define { name, var_type } => {
                let value = if var_type == "num" {
                    Variable::Number {
                        value: 0
                    }
                } else if var_type == "String" {
                    Variable::Text {
                        value: String::default()
                    }
                } else {
                    return Err(format!("{}: 未知の型です。", var_type));
                };
                self.l_vars.insert(
                    name.clone(),
                    LocalVariable {
                        name,
                        value,
                        status: VariableStatus::default(),
                    },
                );
                return Ok(None);
            }
            _ => { Ok(None) },
        }
    }

    pub fn run(&mut self, function_name: String, function_args: Vec<Box<Node>>) -> Result<Variable, String> {
        debug!("searching {}", function_name);

        if let Some(functions) = self.functions.clone() {
            for function in functions {
                match *function {
                    Node::Function { name, body, args } => {
                        if name == function_name {
                            debug!("running: {}", function_name);
                            for program in body {
                                if let Ok(Some(res)) = self.evaluate(program) {
                                    match res {
                                        Variable::Return { value } => {
                                            return Ok(*value);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }

        Ok(Variable::None {})
    }
}
