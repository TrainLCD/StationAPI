pub fn delete_option_from_string_vec(vec: Vec<Option<String>>) -> Vec<String> {
    vec.into_iter()
        .filter_map(|opt| {
            if opt.is_some() {
                return opt;
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn delete_option_from_string_vec() {
        use super::*;

        let vec = vec![Some("a".to_string()), None, Some("b".to_string())];
        let vec = delete_option_from_string_vec(vec);
        assert_eq!(vec, vec!["a".to_string(), "b".to_string()]);
    }
}
