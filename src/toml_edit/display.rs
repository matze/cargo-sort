use std::fmt::{Display, Formatter, Result, Write};

use super::{
    decor::{Formatted, Repr},
    document::Document,
    table::{Item, Table},
    value::{Array, DateTime, InlineTable, Value},
};

impl Display for Repr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{}{}", self.decor.prefix, self.raw_value, self.decor.suffix)
    }
}

impl<T> Display for Formatted<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.repr) }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DateTime::OffsetDateTime(d) => write!(f, "{}", d),
            DateTime::LocalDateTime(d) => write!(f, "{}", d),
            DateTime::LocalDate(d) => write!(f, "{}", d),
            DateTime::LocalTime(d) => write!(f, "{}", d),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Value::Integer(repr) => write!(f, "{}", repr),
            Value::String(repr) => write!(f, "{}", repr),
            Value::Float(repr) => write!(f, "{}", repr),
            Value::Boolean(repr) => write!(f, "{}", repr),
            Value::DateTime(repr) => write!(f, "{}", repr),
            Value::Array(array) => write!(f, "{}", array),
            Value::InlineTable(table) => write!(f, "{}", table),
        }
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let len = self.len().saturating_sub(1);
        write!(f, "{}[", self.decor.prefix)?;
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }

            // The last values decor.suffix is \n so we can't just
            // write a `,` after we write the value
            let mut v2 = v.clone();
            let val = if i == len && self.trailing_comma && self.newlines {
                v2.decor_mut().suffix.insert(0, ',');
                &v2
            } else {
                v
            };

            write!(f, "{}", val)?;
        }

        if self.trailing_comma && !self.newlines {
            write!(f, ",")?;
        }

        write!(f, "{}", self.trailing)?;
        write!(f, "]{}", self.decor.suffix)
    }
}

impl Display for InlineTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{{", self.decor.prefix)?;
        write!(f, "{}", self.preamble)?;
        for (i, (key, value)) in self
            .items
            .iter()
            .filter(|&(_, kv)| kv.value.is_value())
            .map(|(_, kv)| (&kv.key, kv.value.as_value().unwrap()))
            .enumerate()
        {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}={}", key, value)?;
        }
        write!(f, "}}{}", self.decor.suffix)
    }
}

impl Table {
    fn visit_nested_tables<'t, F>(
        &'t self,
        path: &mut Vec<&'t str>,
        is_array_of_tables: bool,
        callback: &mut F,
    ) -> Result
    where
        F: FnMut(&Table, &Vec<&'t str>, bool) -> Result,
    {
        callback(self, path, is_array_of_tables)?;

        for kv in self.items.values() {
            match kv.value {
                Item::Table(ref t) => {
                    path.push(&kv.key.raw_value);
                    t.visit_nested_tables(path, false, callback)?;
                    path.pop();
                }
                Item::ArrayOfTables(ref a) => {
                    for t in a.iter() {
                        path.push(&kv.key.raw_value);
                        t.visit_nested_tables(path, true, callback)?;
                        path.pop();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn visit_table(
    f: &mut dyn Write,
    table: &Table,
    path: &[&str],
    is_array_of_tables: bool,
) -> Result {
    if path.is_empty() {
        // don't print header for the root node
    } else if is_array_of_tables {
        write!(f, "{}[[", table.decor.prefix)?;
        write!(f, "{}", path.join("."))?;
        writeln!(f, "]]{}", table.decor.suffix)?;
    } else if !(table.implicit && table.values_len() == 0) {
        write!(f, "{}[", table.decor.prefix)?;
        write!(f, "{}", path.join("."))?;
        writeln!(f, "]{}", table.decor.suffix)?;
    }
    // print table body
    for kv in table.items.values() {
        if let Item::Value(ref value) = kv.value {
            writeln!(f, "{}={}", kv.key, value)?;
        }
    }
    Ok(())
}

impl Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut path = Vec::new();

        self.visit_nested_tables(&mut path, false, &mut |t, path, is_array| {
            visit_table(f, t, path, is_array)
        })?;
        Ok(())
    }
}

impl Document {
    /// Returns a string representation of the TOML document, attempting to keep
    /// the table headers in their original order.
    pub fn to_string_in_original_order(&self) -> String {
        let mut string = String::new();
        let mut path = Vec::new();
        let mut last_position = 0;
        let mut tables = Vec::new();
        self.as_table()
            .visit_nested_tables(&mut path, false, &mut |t, p, is_array| {
                if let Some(pos) = t.position {
                    last_position = pos;
                }
                let mut s = String::new();
                visit_table(&mut s, t, p, is_array)?;
                tables.push((last_position, s));
                Ok(())
            })
            .unwrap();

        tables.sort_by_key(|&(id, _)| id);
        for (_, table) in tables {
            string.push_str(&table);
        }
        string.push_str(&self.trailing);
        string
    }
}

impl Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.as_table())?;
        write!(f, "{}", self.trailing)
    }
}
