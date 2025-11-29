#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Result {
    Ok(i64),
    Err(String),
}

