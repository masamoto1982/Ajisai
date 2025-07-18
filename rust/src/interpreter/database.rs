use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, TableData};

pub fn op_table(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            if let Some(table) = interp.tables.get(&name) {
                let table_vec = table_to_vector(table);
                interp.stack.push(table_vec);
                interp.current_table = Some(name);
                Ok(())
            } else {
                Err(AjisaiError::from(format!("Table '{}' not found", name)))
            }
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

pub fn op_table_create(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let name_val = interp.stack.pop().unwrap();
    let schema_val = interp.stack.pop().unwrap();

    match (name_val.val_type, schema_val.val_type) {
        (ValueType::String(name), ValueType::Vector(schema_vec)) => {
            let schema: Vec<String> = schema_vec.into_iter()
                .filter_map(|v| {
                    if let ValueType::String(s) = v.val_type { 
                        Some(s) 
                    } else { 
                        None 
                    }
                })
                .collect();
            
            let table_data = TableData { 
                schema, 
                records: Vec::new() 
            };
            interp.tables.insert(name, table_data);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector and string", "other types")),
    }
}

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let filter_quotation = interp.stack.pop().unwrap();
    let table_val = interp.stack.pop().unwrap();

    match (table_val.val_type, filter_quotation.val_type) {
        (ValueType::Vector(records), ValueType::Quotation(filter_tokens)) => {
            let mut filtered_records = Vec::new();
            
            for record_val in records {
                if let ValueType::Vector(_) = &record_val.val_type {
                    interp.stack.push(record_val.clone());
                    interp.execute_tokens_with_context(&filter_tokens)?;
                    
                    if let Some(result) = interp.stack.pop() {
                        if let ValueType::Boolean(true) = result.val_type {
                            filtered_records.push(record_val);
                        }
                    }
                }
            }
            
            interp.stack.push(Value { 
                val_type: ValueType::Vector(filtered_records) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("vector and quotation", "other types")),
    }
}

pub fn op_project(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let columns_val = interp.stack.pop().unwrap();
    let table_val = interp.stack.pop().unwrap();
    
    match (&table_val.val_type, &columns_val.val_type) {
        (ValueType::Vector(_), ValueType::Vector(_)) => {
            // 簡易実装：テーブルをそのまま返す
            interp.stack.push(table_val);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("two vectors", "other types")),
    }
}

pub fn op_insert(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let table_name_val = interp.stack.pop().unwrap();
    let record_val = interp.stack.pop().unwrap();

    match (table_name_val.val_type, record_val.val_type) {
        (ValueType::String(name), ValueType::Vector(fields)) => {
            if let Some(table) = interp.tables.get_mut(&name) {
                table.records.push(fields);
                Ok(())
            } else {
                Err(AjisaiError::from(format!("Table '{}' not found", name)))
            }
        },
        _ => Err(AjisaiError::type_error("vector and string", "other types")),
    }
}

pub fn op_update(_interp: &mut Interpreter) -> Result<()> { 
    // TODO: 実装
    Ok(()) 
}

pub fn op_delete(_interp: &mut Interpreter) -> Result<()> { 
    // TODO: 実装
    Ok(()) 
}

pub fn op_tables(interp: &mut Interpreter) -> Result<()> {
    let pattern_val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match pattern_val.val_type {
        ValueType::String(pattern) => {
            let table_names: Vec<Value> = interp.tables.keys()
                .filter(|name| wildcard_match(name, &pattern))
                .map(|name| Value { 
                    val_type: ValueType::String(name.clone()) 
                })
                .collect();
            
            interp.stack.push(Value { 
                val_type: ValueType::Vector(table_names) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

pub fn op_tables_info(interp: &mut Interpreter) -> Result<()> {
    let mut output = String::new();
    
    if interp.tables.is_empty() {
        output.push_str("No tables found.\n");
    } else {
        output.push_str(&format!("Total tables: {}\n", interp.tables.len()));
        for (name, table) in &interp.tables {
            output.push_str(&format!(
                "Table '{}': {} records, schema: {:?}\n",
                name, table.records.len(), table.schema
            ));
        }
    }
    
    interp.append_output(&output);
    Ok(())
}

pub fn op_table_info(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let mut output = String::new();
            
            if let Some(table) = interp.tables.get(&name) {
                output.push_str(&format!(
                    "Table '{}'\n  Schema: {:?}\n  Records: {}\n",
                    name, table.schema, table.records.len()
                ));
                
                for (i, record) in table.records.iter().take(3).enumerate() {
                    output.push_str(&format!("  Record {}: ", i));
                    for (j, field) in record.iter().enumerate() {
                        if j > 0 { output.push_str(", "); }
                        output.push_str(&format!("{}", field));
                    }
                    output.push('\n');
                }
                
                if table.records.len() > 3 {
                    output.push_str(&format!("  ... and {} more records\n", table.records.len() - 3));
                }
            } else {
                output.push_str(&format!("Table '{}' not found\n", name));
            }
            
            interp.append_output(&output);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

pub fn op_table_size(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let size = interp.tables.get(&name)
                .map_or(0, |t| t.records.len() as i64);
            
            interp.stack.push(Value { 
                val_type: ValueType::Number(Fraction::new(size, 1)) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}

pub fn op_save_db(_interp: &mut Interpreter) -> Result<()> {
    if let Some(window) = web_sys::window() {
        let event = web_sys::CustomEvent::new("ajisai-save-db")
            .map_err(|_| AjisaiError::from("Failed to create save event"))?;
        window.dispatch_event(&event)
            .map_err(|_| AjisaiError::from("Failed to dispatch save event"))?;
    }
    Ok(())
}

pub fn op_load_db(_interp: &mut Interpreter) -> Result<()> {
    if let Some(window) = web_sys::window() {
        let event = web_sys::CustomEvent::new("ajisai-load-db")
            .map_err(|_| AjisaiError::from("Failed to create load event"))?;
        window.dispatch_event(&event)
            .map_err(|_| AjisaiError::from("Failed to dispatch load event"))?;
    }
    Ok(())
}

pub fn op_match(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let pattern = interp.stack.pop().unwrap();
    let value = interp.stack.pop().unwrap();
    
    match (value.val_type, pattern.val_type) {
        (ValueType::String(s), ValueType::String(p)) => {
            interp.stack.push(Value { 
                val_type: ValueType::Boolean(wildcard_match(&s, &p)) 
            });
            Ok(())
        },
        _ => Err(AjisaiError::type_error("two strings", "other types")),
    }
}

pub fn op_wildcard(_interp: &mut Interpreter) -> Result<()> {
    // パターンをそのまま使うので、特に処理は不要
    Ok(())
}

// ヘルパー関数
fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    
    if !pattern.contains('*') && !pattern.contains('?') {
        return text == pattern;
    }
    
    // 簡易実装
    let pattern_without_wildcards = pattern.replace("*", "").replace("?", "");
    text.contains(&pattern_without_wildcards)
}

fn table_to_vector(table: &TableData) -> Value {
    Value {
        val_type: ValueType::Vector(
            table.records.iter()
                .map(|rec| Value { 
                    val_type: ValueType::Vector(rec.clone()) 
                })
                .collect()
        )
    }
}
