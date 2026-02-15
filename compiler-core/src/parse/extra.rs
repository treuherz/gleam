use std::cmp::Ordering;

use ecow::EcoString;

use crate::ast::SrcSpan;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct ModuleExtra {
    pub module_comments: Vec<SrcSpan>,
    pub doc_comments: Vec<SrcSpan>,
    pub comments: Vec<SrcSpan>,
    pub empty_lines: Vec<u32>,
    pub new_lines: Vec<u32>,
    pub trailing_commas: Vec<u32>,
}

impl ModuleExtra {
    pub fn new() -> Self {
        Default::default()
    }

    /// Detects if a byte index is in a comment context
    pub fn is_within_comment(&self, byte_index: u32) -> bool {
        let cmp = |span: &SrcSpan| {
            if byte_index < span.start {
                Ordering::Greater
            } else if byte_index > span.end {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        };

        self.comments.binary_search_by(cmp).is_ok()
            || self.doc_comments.binary_search_by(cmp).is_ok()
            || self.module_comments.binary_search_by(cmp).is_ok()
    }

    pub fn has_comment_between(&self, start: u32, end: u32) -> bool {
        self.first_comment_between(start, end).is_some()
    }

    /// Returns the first comment overlapping the given source locations (inclusive)
    /// Note that the returned span covers the text of the comment, not the `//`
    pub fn first_comment_between(&self, start: u32, end: u32) -> Option<SrcSpan> {
        // Helper function to find a comment that is between the given start
        // and end. Not guaranteed to find the first comment.
        let find_comment_between = |comments: &[SrcSpan], start, end| -> Option<usize> {
            if comments.is_empty() {
                return None;
            }

            comments
                .binary_search_by(|comment| {
                    if comment.end < start {
                        Ordering::Less
                    } else if comment.start > end {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                })
                .ok()
        };

        // To find the first comment in the given span, we first see if we can
        // find any comment at all in the span by binary-searching over the list
        // of comments in the module. If we do, we need to see if any other
        // comment appears earlier, so we do the same search using the sub-list
        // of comments before the one we found.
        //
        // We repeat this, narrowing our search list each time, until we can't
        // find any comment earlier than our best.
        let mut first_index_so_far = None;
        let mut search_list = &self.comments[..];
        while let Some(index) = find_comment_between(search_list, start, end) {
            first_index_so_far = Some(index);
            search_list = search_list.get(0..index).unwrap_or(&[]);
        }

        first_index_so_far
            .and_then(|index| self.comments.get(index))
            .copied()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Comment<'a> {
    pub start: u32,
    pub content: &'a str,
}

impl<'a> From<(&SrcSpan, &'a EcoString)> for Comment<'a> {
    fn from(value: (&SrcSpan, &'a EcoString)) -> Self {
        Self::from((value.0, value.1.as_str()))
    }
}

impl<'a> From<(&SrcSpan, &'a str)> for Comment<'a> {
    fn from(src: (&SrcSpan, &'a str)) -> Comment<'a> {
        let start = src.0.start;
        let end = src.0.end as usize;
        Comment {
            start,
            content: src
                .1
                .get(start as usize..end)
                .expect("From span to comment"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::SrcSpan, parse::extra::ModuleExtra};

    fn set_up_extra() -> ModuleExtra {
        let mut extra = ModuleExtra::new();
        extra.comments = vec![
            SrcSpan { start: 0, end: 10 },
            SrcSpan { start: 20, end: 30 },
            SrcSpan { start: 40, end: 50 },
            SrcSpan { start: 60, end: 70 },
            SrcSpan { start: 80, end: 90 },
            SrcSpan {
                start: 90,
                end: 100,
            },
        ];
        extra
    }

    #[test]
    fn first_comment_between() {
        let extra = set_up_extra();
        assert!(matches!(
            extra.first_comment_between(15, 85),
            Some(SrcSpan { start: 20, end: 30 })
        ));
    }

    #[test]
    fn first_comment_between_equal_to_range() {
        let extra = set_up_extra();
        assert!(matches!(
            extra.first_comment_between(40, 50),
            Some(SrcSpan { start: 40, end: 50 })
        ));
    }

    #[test]
    fn first_comment_between_overlapping_start_of_range() {
        let extra = set_up_extra();
        assert!(matches!(
            extra.first_comment_between(45, 80),
            Some(SrcSpan { start: 40, end: 50 })
        ));
    }

    #[test]
    fn first_comment_between_overlapping_end_of_range() {
        let extra = set_up_extra();
        assert!(matches!(
            extra.first_comment_between(35, 45),
            Some(SrcSpan { start: 40, end: 50 })
        ));
    }

    #[test]
    fn first_comment_between_at_end_of_range() {
        let extra = set_up_extra();
        assert!(matches!(
            dbg!(extra.first_comment_between(55, 60)),
            Some(SrcSpan { start: 60, end: 70 })
        ));
    }
}
