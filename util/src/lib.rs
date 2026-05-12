use std::{fs::File, io};

use serde::{Deserialize, Serialize};

pub fn main() {
    println!("Hello, world!");
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Edge {
    from: u32,
    to: u32,
}

impl Edge {
    pub fn new(from: u32, to: u32) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Counter {
    id: u32,
    triple: u32,
    sequence: u32,
    neighbor: u32,
    skip: u32,
    double: u32,
}

impl Counter {
    pub fn new(id: u32, triple: u32, sequence: u32, neighbor: u32, skip: u32, double: u32) -> Self {
        Self {
            id,
            triple,
            sequence,
            neighbor,
            skip,
            double,
        }
    }
}

pub fn print<V: Serialize>(records: impl Iterator<Item = V>, path: &str) -> Result<(), io::Error> {
    let mut wtr = csv::Writer::from_writer(File::create(path)?);
    for record in records {
        wtr.serialize(record)?;
    }
    wtr.flush()?;
    Ok(())
}
