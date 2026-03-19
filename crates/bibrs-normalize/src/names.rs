/// Decomposed BibTeX author/editor name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonName {
    pub first: String,
    pub von: String,
    pub last: String,
    pub jr: String,
    pub is_institutional: bool,
}

impl PersonName {
    /// Formats as "Last, First" (default BibTeX style).
    pub fn to_last_comma_first(&self) -> String {
        let mut parts = Vec::new();
        let mut last_part = String::new();
        if !self.von.is_empty() {
            last_part.push_str(&self.von);
            last_part.push(' ');
        }
        last_part.push_str(&self.last);
        parts.push(last_part);
        if !self.jr.is_empty() {
            parts.push(self.jr.clone());
        }
        if !self.first.is_empty() {
            parts.push(self.first.clone());
        }
        parts.join(", ")
    }

    /// Formats as "First Last".
    pub fn to_first_last(&self) -> String {
        let mut parts = Vec::new();
        if !self.first.is_empty() {
            parts.push(self.first.clone());
        }
        if !self.von.is_empty() {
            parts.push(self.von.clone());
        }
        parts.push(self.last.clone());
        if !self.jr.is_empty() {
            parts.push(self.jr.clone());
        }
        parts.join(" ")
    }
}

/// Splits a BibTeX author/editor field on " and " into individual name strings.
pub fn split_authors(input: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0u32;
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' if depth > 0 => depth -= 1,
            b' ' if depth == 0 && i + 5 <= bytes.len() => {
                if input.get(i..i + 5) == Some(" and ") {
                    let name = input[start..i].trim();
                    if !name.is_empty() {
                        result.push(name);
                    }
                    i += 5;
                    start = i;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let last = input[start..].trim();
    if !last.is_empty() {
        result.push(last);
    }
    result
}

/// Parses a single BibTeX name into a PersonName.
///
/// Handles formats:
/// - "Last, First"
/// - "Last, Jr., First"
/// - "First Last"
/// - "First Middle Last"
/// - "{Institutional Name}"
/// - "von Last, First" (particle detection)
pub fn parse_name(input: &str) -> PersonName {
    let trimmed = input.trim();

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return PersonName {
            first: String::new(),
            von: String::new(),
            last: trimmed[1..trimmed.len() - 1].to_string(),
            jr: String::new(),
            is_institutional: true,
        };
    }

    let comma_parts = split_on_commas(trimmed);

    match comma_parts.len() {
        1 => parse_first_last(comma_parts[0]),
        2 => {
            let (von, last) = split_von_last(comma_parts[0]);
            PersonName {
                first: comma_parts[1].trim().to_string(),
                von,
                last,
                jr: String::new(),
                is_institutional: false,
            }
        }
        _ => {
            let (von, last) = split_von_last(comma_parts[0]);
            PersonName {
                first: comma_parts[2].trim().to_string(),
                von,
                last,
                jr: comma_parts[1].trim().to_string(),
                is_institutional: false,
            }
        }
    }
}

/// Parses an entire author/editor field into a list of names.
pub fn parse_authors(input: &str) -> Vec<PersonName> {
    split_authors(input).into_iter().map(parse_name).collect()
}

fn split_on_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0u32;

    for (i, b) in input.bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' if depth > 0 => depth -= 1,
            b',' if depth == 0 => {
                parts.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn parse_first_last(input: &str) -> PersonName {
    let words = split_words(input);
    if words.is_empty() {
        return PersonName {
            first: String::new(),
            von: String::new(),
            last: String::new(),
            jr: String::new(),
            is_institutional: false,
        };
    }
    if words.len() == 1 {
        return PersonName {
            first: String::new(),
            von: String::new(),
            last: words[0].to_string(),
            jr: String::new(),
            is_institutional: false,
        };
    }

    let mut first_end = 0;
    let mut von_start = words.len();
    let mut von_end = words.len();

    for (i, word) in words.iter().enumerate() {
        if i == words.len() - 1 {
            break;
        }
        if is_von_particle(word) {
            von_start = i;
            break;
        }
        first_end = i + 1;
    }

    if von_start < words.len() - 1 {
        for i in (von_start..words.len() - 1).rev() {
            if !is_von_particle(words[i]) {
                von_end = i + 1;
                break;
            }
        }
        if von_end == words.len() {
            von_end = words.len() - 1;
        }
    }

    let first = words[..first_end].join(" ");
    let von = if von_start < von_end {
        words[von_start..von_end].join(" ")
    } else {
        String::new()
    };
    let last_start = if von.is_empty() {
        words.len() - 1
    } else {
        von_end
    };
    let last = words[last_start..].join(" ");

    PersonName {
        first,
        von,
        last,
        jr: String::new(),
        is_institutional: false,
    }
}

fn split_von_last(input: &str) -> (String, String) {
    let words = split_words(input.trim());
    if words.is_empty() {
        return (String::new(), String::new());
    }
    if words.len() == 1 {
        return (String::new(), words[0].to_string());
    }

    let mut von_end = 0;
    for (i, word) in words.iter().enumerate() {
        if i == words.len() - 1 {
            break;
        }
        if is_von_particle(word) {
            von_end = i + 1;
        }
    }

    if von_end == 0 {
        (String::new(), words.join(" "))
    } else {
        let von = words[..von_end].join(" ");
        let last = words[von_end..].join(" ");
        (von, last)
    }
}

fn split_words(input: &str) -> Vec<&str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut words = Vec::new();
    let mut start = 0;
    let mut depth = 0u32;
    let bytes = trimmed.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' if depth > 0 => depth -= 1,
            b' ' | b'~' if depth == 0 => {
                let word = &trimmed[start..i];
                if !word.is_empty() {
                    words.push(word);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let word = &trimmed[start..];
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn is_von_particle(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    if word.starts_with('{') {
        return false;
    }
    word.chars()
        .next()
        .map(|c| c.is_lowercase())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_last_comma_first() {
        let name = parse_name("Silva, João");
        assert_eq!(name.last, "Silva");
        assert_eq!(name.first, "João");
        assert!(name.von.is_empty());
        assert!(!name.is_institutional);
    }

    #[test]
    fn simple_first_last() {
        let name = parse_name("João Silva");
        assert_eq!(name.first, "João");
        assert_eq!(name.last, "Silva");
    }

    #[test]
    fn first_middle_last() {
        let name = parse_name("João Carlos Silva");
        assert_eq!(name.first, "João Carlos");
        assert_eq!(name.last, "Silva");
    }

    #[test]
    fn last_jr_first() {
        let name = parse_name("Smith, Jr., John");
        assert_eq!(name.last, "Smith");
        assert_eq!(name.jr, "Jr.");
        assert_eq!(name.first, "John");
    }

    #[test]
    fn institutional() {
        let name = parse_name("{World Health Organization}");
        assert_eq!(name.last, "World Health Organization");
        assert!(name.is_institutional);
    }

    #[test]
    fn von_particle_last_first() {
        let name = parse_name("van der Berg, Jan");
        assert_eq!(name.von, "van der");
        assert_eq!(name.last, "Berg");
        assert_eq!(name.first, "Jan");
    }

    #[test]
    fn von_particle_first_last() {
        let name = parse_name("Jan van der Berg");
        assert_eq!(name.first, "Jan");
        assert_eq!(name.von, "van der");
        assert_eq!(name.last, "Berg");
    }

    #[test]
    fn de_particle() {
        let name = parse_name("de Souza, Maria");
        assert_eq!(name.von, "de");
        assert_eq!(name.last, "Souza");
        assert_eq!(name.first, "Maria");
    }

    #[test]
    fn multiple_authors_split() {
        let names = parse_authors("Silva, João and Santos, Maria and {CERN}");
        assert_eq!(names.len(), 3);
        assert_eq!(names[0].last, "Silva");
        assert_eq!(names[1].last, "Santos");
        assert_eq!(names[2].last, "CERN");
        assert!(names[2].is_institutional);
    }

    #[test]
    fn single_name() {
        let name = parse_name("Aristotle");
        assert_eq!(name.last, "Aristotle");
        assert!(name.first.is_empty());
    }

    #[test]
    fn von_complex() {
        let name = parse_name("Ludwig van Beethoven");
        assert_eq!(name.first, "Ludwig");
        assert_eq!(name.von, "van");
        assert_eq!(name.last, "Beethoven");
    }

    #[test]
    fn suffix_iii() {
        let name = parse_name("Gates, III, William");
        assert_eq!(name.last, "Gates");
        assert_eq!(name.jr, "III");
        assert_eq!(name.first, "William");
    }

    #[test]
    fn format_last_comma_first() {
        let name = parse_name("van der Berg, Jan");
        assert_eq!(name.to_last_comma_first(), "van der Berg, Jan");
    }

    #[test]
    fn format_first_last() {
        let name = parse_name("Silva, João");
        assert_eq!(name.to_first_last(), "João Silva");
    }

    #[test]
    fn real_names_batch() {
        let test_cases = vec![
            ("Knuth, Donald E.", "Knuth", "Donald E."),
            ("Donald E. Knuth", "Knuth", "Donald E."),
            ("Lamport, Leslie", "Lamport", "Leslie"),
            ("Turing, Alan M.", "Turing", "Alan M."),
            ("Alan M. Turing", "Turing", "Alan M."),
            ("von Neumann, John", "Neumann", "John"),
            ("John von Neumann", "Neumann", "John"),
            ("de la Cruz, Ana", "Cruz", "Ana"),
            ("Ana de la Cruz", "Cruz", "Ana"),
            ("van Rossum, Guido", "Rossum", "Guido"),
            ("Guido van Rossum", "Rossum", "Guido"),
            ("Dijkstra, Edsger W.", "Dijkstra", "Edsger W."),
            ("Edsger W. Dijkstra", "Dijkstra", "Edsger W."),
            ("Rivest, Ronald L.", "Rivest", "Ronald L."),
            ("Shannon, Claude E.", "Shannon", "Claude E."),
            ("Berners-Lee, Tim", "Berners-Lee", "Tim"),
            ("Tim Berners-Lee", "Berners-Lee", "Tim"),
            ("Hopcroft, John E.", "Hopcroft", "John E."),
            ("Ullman, Jeffrey D.", "Ullman", "Jeffrey D."),
            ("Cormen, Thomas H.", "Cormen", "Thomas H."),
            ("Leiserson, Charles E.", "Leiserson", "Charles E."),
            ("Sipser, Michael", "Sipser", "Michael"),
            ("Tanenbaum, Andrew S.", "Tanenbaum", "Andrew S."),
            ("Stallings, William", "Stallings", "William"),
            ("Sedgewick, Robert", "Sedgewick", "Robert"),
            ("Aho, Alfred V.", "Aho", "Alfred V."),
            ("Wirth, Niklaus", "Wirth", "Niklaus"),
            ("Hoare, C. A. R.", "Hoare", "C. A. R."),
            ("Ritchie, Dennis M.", "Ritchie", "Dennis M."),
            ("Thompson, Ken", "Thompson", "Ken"),
            ("Stroustrup, Bjarne", "Stroustrup", "Bjarne"),
            ("Gosling, James", "Gosling", "James"),
            ("Wall, Larry", "Wall", "Larry"),
            ("Matsumoto, Yukihiro", "Matsumoto", "Yukihiro"),
            ("Torvalds, Linus", "Torvalds", "Linus"),
            ("Stallman, Richard M.", "Stallman", "Richard M."),
            ("Raymond, Eric S.", "Raymond", "Eric S."),
            ("Cerf, Vinton G.", "Cerf", "Vinton G."),
            ("Kahn, Robert E.", "Kahn", "Robert E."),
            ("Postel, Jon", "Postel", "Jon"),
            ("Clark, David D.", "Clark", "David D."),
            ("Floyd, Robert W.", "Floyd", "Robert W."),
            ("Tarjan, Robert E.", "Tarjan", "Robert E."),
            ("Garey, Michael R.", "Garey", "Michael R."),
            ("Johnson, David S.", "Johnson", "David S."),
            ("Cook, Stephen A.", "Cook", "Stephen A."),
            ("Karp, Richard M.", "Karp", "Richard M."),
            ("Valiant, Leslie G.", "Valiant", "Leslie G."),
            ("Yao, Andrew Chi-Chih", "Yao", "Andrew Chi-Chih"),
            ("Goldwasser, Shafi", "Goldwasser", "Shafi"),
        ];

        for (input, expected_last, expected_first) in test_cases {
            let name = parse_name(input);
            assert_eq!(
                name.last, expected_last,
                "Failed on '{}': expected last='{}', got last='{}'",
                input, expected_last, name.last
            );
            assert_eq!(
                name.first, expected_first,
                "Failed on '{}': expected first='{}', got first='{}'",
                input, expected_first, name.first
            );
        }
    }

    #[test]
    fn institutional_names_batch() {
        let cases = vec![
            "{World Health Organization}",
            "{National Institute of Standards and Technology}",
            "{European Organization for Nuclear Research}",
            "{International Organization for Standardization}",
            "{United Nations}",
        ];
        for input in cases {
            let name = parse_name(input);
            assert!(name.is_institutional, "Failed on '{}'", input);
        }
    }

    #[test]
    fn von_particles_batch() {
        let cases = vec![
            ("von Neumann, John", "von", "Neumann"),
            ("van der Waals, Johannes", "van der", "Waals"),
            ("de la Vallée Poussin, Charles-Jean", "de la", "Vallée Poussin"),
            ("di Francesco, Pierre", "di", "Francesco"),
            ("da Silva, José", "da", "Silva"),
            ("dos Santos, Carlos", "dos", "Santos"),
            ("delle Fave, Giovanni", "delle", "Fave"),
            ("ten Bosch, Hendrik", "ten", "Bosch"),
            ("ter Haar, Dick", "ter", "Haar"),
            ("zum Busch, Friedrich", "zum", "Busch"),
        ];
        for (input, expected_von, expected_last) in cases {
            let name = parse_name(input);
            assert_eq!(
                name.von, expected_von,
                "Failed on '{}': expected von='{}', got von='{}'",
                input, expected_von, name.von
            );
            assert_eq!(
                name.last, expected_last,
                "Failed on '{}': expected last='{}', got last='{}'",
                input, expected_last, name.last
            );
        }
    }
}
