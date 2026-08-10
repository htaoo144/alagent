pub fn fixed_length_chunking(
    text:&str,
    chunk_size:usize,
    overlap:usize,
)->Vec<String>{
    if chunk_size == 0 {
        return vec![];
    }
    assert!(overlap < text.len());

    let chars = text.chars().collect::<Vec<char>>();
    if chars.is_empty(){
        return vec![];
    }

    let mut chunk = Vec::new();
    let mut start = 0;

    while start < chars.len(){
        let end = (start + overlap).min(chars.len());

        let piece =chars[start..end].iter().collect::<String>();

        if !piece.trim().is_empty() {
            chunk.push(piece);
        }
        if end ==chunk.len(){
            break;
        }
        start = end-overlap;
    }
    chunk
}

