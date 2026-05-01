#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone)]
pub enum Cell {
    Null,
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Bytes(Vec<u8>),
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::Null => write!(f, "NULL"),
            Cell::Text(s) => write!(f, "{}", s),
            Cell::Integer(i) => write!(f, "{}", i),
            Cell::Float(fl) => write!(f, "{}", fl),
            Cell::Boolean(b) => write!(f, "{}", b),
            Cell::Bytes(_) => write!(f, "<bytes>"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SortState {
    pub column: usize,
    pub direction: SortDirection,
}

#[derive(Debug, Clone)]
pub enum SortDirection {
    Asc,
    Desc,
}
