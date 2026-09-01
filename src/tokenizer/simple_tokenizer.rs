use std::str::CharIndices;

use super::{Token, TokenStream, Tokenizer};

/// Tokenize the text by splitting on whitespaces and punctuation.
#[derive(Clone, Default)]
pub struct SimpleTokenizer {
    token: Token,
}

/// TokenStream produced by the `SimpleTokenizer`.
pub struct SimpleTokenStream<'a> {
    text: &'a str,
    chars: CharIndices<'a>,
    token: &'a mut Token,
}

/// Whether a character belongs to the word being read.
///
/// A letter or a digit does, and so does a mark written on top of one: in
/// Devanagari and Bengali the sign that joins two consonants is a mark of its
/// own, and a tokenizer that stopped at it would cut a word in half.
#[inline]
fn part_of_word(c: char) -> bool {
    c.is_alphanumeric() || is_combining_mark(c)
}

/// The ranges of characters that are written onto another one.
#[inline]
fn is_combining_mark(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}'   // the Latin ones
        | '\u{0483}'..='\u{0489}'
        | '\u{0591}'..='\u{05BD}'
        | '\u{0610}'..='\u{061A}'
        | '\u{064B}'..='\u{065F}' // Arabic
        | '\u{0670}'
        | '\u{06D6}'..='\u{06DC}'
        | '\u{0900}'..='\u{0903}' // Devanagari
        | '\u{093A}'..='\u{094F}'
        | '\u{0951}'..='\u{0957}'
        | '\u{0962}'..='\u{0963}'
        | '\u{0981}'..='\u{0983}' // Bengali
        | '\u{09BC}'..='\u{09CD}'
        | '\u{09D7}'
        | '\u{09E2}'..='\u{09E3}'
        | '\u{0A01}'..='\u{0A03}' // the other Indic scripts
        | '\u{0A3C}'..='\u{0A51}'
        | '\u{0A81}'..='\u{0A83}'
        | '\u{0ABC}'..='\u{0ACD}'
        | '\u{0B01}'..='\u{0B03}'
        | '\u{0B3C}'..='\u{0B57}'
        | '\u{0B82}'
        | '\u{0BBE}'..='\u{0BCD}'
        | '\u{0C00}'..='\u{0C04}'
        | '\u{0C3E}'..='\u{0C56}'
        | '\u{0C81}'..='\u{0C83}'
        | '\u{0CBC}'..='\u{0CD6}'
        | '\u{0D00}'..='\u{0D03}'
        | '\u{0D3B}'..='\u{0D57}'
        | '\u{0E31}'                // Thai
        | '\u{0E34}'..='\u{0E3A}'
        | '\u{0E47}'..='\u{0E4E}'
        | '\u{1AB0}'..='\u{1AFF}'
        | '\u{20D0}'..='\u{20F0}'
        | '\u{FE20}'..='\u{FE2F}'
    )
}

impl Tokenizer for SimpleTokenizer {
    type TokenStream<'a> = SimpleTokenStream<'a>;
    fn token_stream<'a>(&'a mut self, text: &'a str) -> SimpleTokenStream<'a> {
        self.token.reset();
        SimpleTokenStream {
            text,
            chars: text.char_indices(),
            token: &mut self.token,
        }
    }
}

impl SimpleTokenStream<'_> {
    // search for the end of the current token.
    fn search_token_end(&mut self) -> usize {
        (&mut self.chars)
            .filter(|(_, c)| !part_of_word(*c))
            .map(|(offset, _)| offset)
            .next()
            .unwrap_or(self.text.len())
    }
}

impl TokenStream for SimpleTokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.token.text.clear();
        self.token.position = self.token.position.wrapping_add(1);
        while let Some((offset_from, c)) = self.chars.next() {
            if part_of_word(c) {
                let offset_to = self.search_token_end();
                self.token.offset_from = offset_from;
                self.token.offset_to = offset_to;
                self.token.text.push_str(&self.text[offset_from..offset_to]);
                return true;
            }
        }
        false
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

#[cfg(test)]
mod mark_tests {
    use super::*;
    use crate::tokenizer::{TextAnalyzer, Token};

    fn tokens(text: &str) -> Vec<String> {
        let mut analyzer = TextAnalyzer::from(SimpleTokenizer::default());
        let mut stream = analyzer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            let Token { text, .. } = stream.token();
            out.push(text.clone());
        }
        out
    }

    #[test]
    fn a_joined_consonant_does_not_end_the_word() {
        // Devanagari: the sign between the two consonants belongs to the word
        assert_eq!(tokens("\u{0939}\u{093F}\u{0928}\u{094D}\u{0926}\u{0940}").len(), 1);
        assert_eq!(tokens("hello world"), vec!["hello", "world"]);
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenizer::tests::assert_token;
    use crate::tokenizer::{SimpleTokenizer, TextAnalyzer, Token};

    #[test]
    fn test_simple_tokenizer() {
        let tokens = token_stream_helper("Hello, happy tax payer!");
        assert_eq!(tokens.len(), 4);
        assert_token(&tokens[0], 0, "Hello", 0, 5);
        assert_token(&tokens[1], 1, "happy", 7, 12);
        assert_token(&tokens[2], 2, "tax", 13, 16);
        assert_token(&tokens[3], 3, "payer", 17, 22);
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let mut a = TextAnalyzer::from(SimpleTokenizer::default());
        let mut token_stream = a.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);
        tokens
    }
}
