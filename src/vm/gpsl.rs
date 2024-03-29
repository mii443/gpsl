use crate::external_function::{ExternalFuncReturn, ExternalFuncStatus};
use crate::node::*;
use crate::permission::Permission;
use crate::source::Source;
use crate::variable::*;
use std::collections::{HashMap, VecDeque};
use std::string::*;

#[derive(Clone, Debug)]
pub struct Block {
    pub accept: Vec<Permission>,
    pub reject: Vec<Permission>,
    pub variables: HashMap<String, LocalVariable>,
    pub is_split: bool
}

pub struct GPSL {
    pub functions: Option<HashMap<String, Box<Node>>>,
    pub global_variables: Vec<Variable>,
    pub source: Source,
    pub blocks: VecDeque<Block>,
    pub external_func: Vec<fn(String, Vec<Variable>, Vec<Permission>, Vec<Permission>) -> ExternalFuncReturn>
}

#[derive(Clone, Debug)]
pub struct LocalVariable {
    pub name: String,
    pub value: Variable,
    pub status: VariableStatus,
}

#[derive(Clone, Debug)]
pub struct VariableStatus {
    pub initialized: bool,
}

impl VariableStatus {
    pub fn default() -> VariableStatus {
        VariableStatus { initialized: false }
    }
}

impl GPSL {
    pub fn new(source: Source, functions: Option<HashMap<String, Box<Node>>>, external_func: Vec<fn(String, Vec<Variable>, Vec<Permission>, Vec<Permission>) -> ExternalFuncReturn>) -> GPSL {
        GPSL {
            source,
            functions,
            global_variables: vec![],
            blocks: VecDeque::new(),
            external_func
        }
    }

    pub fn get_local_var_mut(&mut self, name: &String) -> Option<&mut LocalVariable> {
        for x in 0..self.blocks.len() {
            if self.blocks[x].variables.contains_key(name) {
                return self.blocks[x].variables.get_mut(name);
            }

            if self.blocks[x].is_split {
                break
            }
        }
        None
    }

    pub fn get_local_var(&mut self, name: &String) -> Option<LocalVariable> {
        for x in 0..self.blocks.len() {
            if self.blocks[x].variables.contains_key(name) {
                if let Some(var) = self.blocks[x].variables.get(name).clone() {
                    return Some(var.clone());
                } else {
                    return None;
                }
            }

            if self.blocks[x].is_split {
                break
            }
        }
        None
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
                    debug!("functions: {:?}", functions.iter().map(|f| format!("{},", f.0)).collect::<String>());
                    debug!("{}: {}", &function_name, functions.contains_key(&function_name));
                    if functions.contains_key(&function_name) {
                        if let Node::Function { body, .. } = &*(functions[&function_name]) {
                            for program in body {
                                let block = {
                                    let blocks = self.blocks.clone();
                                    blocks.front().unwrap().clone()
                                };

                                self.blocks.push_front(Block {
                                    accept: block.accept.clone(),
                                    reject: block.reject.clone(),
                                    variables: HashMap::new(),
                                    is_split: true
                                });

                                let res = self.evaluate(Box::new(*program.clone()));

                                if let Ok(Some(res)) = res {
                                    match res {
                                        Variable::Return { value } => {
                                            return Ok(Some(*value));
                                        }
                                        _ => {}
                                    }
                                } else if let Err(err) = res {
                                    return Err(err);
                                }

                                self.blocks.pop_front();
                            }
                        }
                        return Ok(None);
                    }
                }

                debug!("Searching external: {}, ({:?})", &function_name, args_value);

                for func in f {
                    let block = self.blocks.front().unwrap();
                    let res = func(function_name.clone(), args_value.clone(), block.accept.clone(), block.reject.clone());
                    if res.status == ExternalFuncStatus::SUCCESS {
                        return Ok(res.value);
                    }
                    if res.status == ExternalFuncStatus::REJECTED {
                        return Err("External function rejected.".to_string());
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
                    debug!("Assign: {:?}", self.blocks.front());

                    let rhs = self.evaluate(rhs);

                    if let Ok(Some(rhs)) = rhs {
                        match *(lhs.clone()) {
                            Node::Lvar { value } => {
                                self.get_local_var_mut(&value).unwrap().value = rhs;
                                self.get_local_var_mut(&value).unwrap().status.initialized = true;
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
                return Ok(Some(self.get_local_var(&value).unwrap().value.clone()));
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
                                Variable::Return { .. } => {
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
                                        Variable::Return { .. } => {
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
            Node::Block { stmts, permission } => {
                let accept = self.blocks.front().unwrap().accept.clone();
                let reject = self.blocks.front().unwrap().reject.clone();
                let (accept, reject) = if let Node::Permission { accept, reject } = *permission.unwrap_or(Box::new(Node::None)) {
                    (accept.iter().map(|p| Permission::from_string(p)).collect(), reject.iter().map(|p| Permission::from_string(p)).collect())
                } else {
                    (accept, reject)
                };

                self.blocks.push_front(Block {
                    accept: accept,
                    reject: reject,
                    variables: HashMap::new(),
                    is_split: false
                });

                for stmt in stmts {
                    let ret = self.evaluate(stmt)?;
                    if let Some(ret) = ret {
                        match ret.clone() {
                            Variable::Return { .. } => {
                                return Ok(Some(ret));
                            }
                            _ => {}
                        }
                    }
                }

                self.blocks.pop_front();

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
                self.blocks.front_mut().unwrap().variables.insert(
                    name.clone(),
                    LocalVariable {
                        name,
                        value,
                        status: VariableStatus::default(),
                    },
                );

                debug!("Define: {:?}", self.blocks.front());

                return Ok(None);
            }
            _ => { Ok(None) },
        }
    }

    pub fn run(&mut self, function_name: String, _: Vec<Box<Node>>) -> Result<Variable, String> {
        debug!("functions: {:?}", self.functions);
        debug!("searching {}", function_name);
        self.blocks.push_front(Block {
            accept: vec![Permission::Administrator, Permission::StdIo],
            reject: vec![],
            variables: HashMap::new(),
            is_split: true
        });
        if let Some(functions) = self.functions.clone() {
            if let Node::Function { body, .. } = &*(functions[&function_name]) {
                for program in body {
                    let res = self.evaluate(Box::new(*program.clone()));
                    if let Ok(Some(res)) = res {
                        match res {
                            Variable::Return { value } => {
                                return Ok(*value);
                            }
                            _ => {}
                        }
                    } else if let Err(err) = res {
                        return Err(err);
                    }
                }
            }
        }

        Ok(Variable::None {})
    }
}
