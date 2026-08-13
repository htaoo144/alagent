pub fn fixed_length_chunking(
    text:&str,
    chunk_size:usize,
    overlap:usize,
)->Vec<String>{
    if chunk_size == 0 || overlap >= chunk_size {
        return vec![];
    }

    let chars = text.chars().collect::<Vec<char>>();
    if chars.is_empty(){
        return vec![];
    }

    let mut chunk = Vec::new();
    let mut start = 0;

    while start < chars.len(){
        let end = (start + chunk_size).min(chars.len());

        let piece =chars[start..end].iter().collect::<String>();

        if !piece.trim().is_empty() {
            chunk.push(piece);
        }
        if end == chars.len(){
            break;
        }
        start = end-overlap;
    }
    chunk
}

#[cfg(test)]
mod tests {
    use super::fixed_length_chunking;

    #[test]
    fn chunks_by_chunk_size_with_overlap() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let chunks = fixed_length_chunking(text, 10, 2);
        assert_eq!(chunks, vec![
            "abcdefghij".to_string(),
            "ijklmnopqr".to_string(),
            "qrstuvwxyz".to_string(),
        ]);
    }

    #[test]
    fn text_shorter_than_chunk_size_returns_single_chunk() {
        let chunks = fixed_length_chunking("hello", 100, 10);
        assert_eq!(chunks, vec!["hello".to_string()]);
    }

    #[test]
    fn overlap_not_smaller_than_chunk_size_returns_empty() {
        assert!(fixed_length_chunking("hello world", 10, 10).is_empty());
    }

    #[test]
    fn empty_or_zero_size_returns_empty() {
        assert!(fixed_length_chunking("", 10, 2).is_empty());
        assert!(fixed_length_chunking("hello", 0, 0).is_empty());
    }
}
