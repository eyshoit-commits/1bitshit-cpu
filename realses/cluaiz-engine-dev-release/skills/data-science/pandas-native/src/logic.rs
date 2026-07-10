// Pandas
pub fn clean_data(csv: &str) -> String { format!("df = pd.read_csv('{}'); df.dropna()", csv) }
