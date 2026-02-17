use crate::logic::registry::CommentType;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(crate) fn count_lines(path: &Path, comments: &CommentType) -> Result<u64, std::io::Error> {
    if !path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));
    }
    let mut inside_block = false;
    let cnt = BufReader::new(File::open(path)?)
        .lines()
        .map_while(Result::ok)
        .filter(|line| is_line_of_code(line, &mut inside_block, comments))
        .count() as u64;

    Ok(cnt)
}

fn is_line_of_code(line: &str, is_inside_block: &mut bool, comment_type: &CommentType) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    !(is_single_line_comment(line, &comment_type.line)
        || is_block_comment(line, is_inside_block, comment_type))
}

fn is_single_line_comment(line: &str, line_comment: &[String]) -> bool {
    for comment_type in line_comment.iter() {
        if line.trim_start().starts_with(comment_type) {
            return true;
        }
    }
    false
}

fn is_block_comment(line: &str, is_inside_block: &mut bool, comment_type: &CommentType) -> bool {
    let mut code_present = false;
    let mut start = 0;
    let trimmed = line.trim();
    let block = match &comment_type.block {
        Some(block) => block,
        _ => return false, // when block comments do not exist always return false
    };

    let open = &block.open;
    let close = &block.close;

    if *is_inside_block {
        if let Some(idx) = trimmed.find(close) {
            start = idx + close.len();
            *is_inside_block = false;
        } else {
            return true;
        }
    }

    while start < trimmed.len() {
        let rest = &trimmed[start..];
        if rest.starts_with(open) {
            start += open.len();
            *is_inside_block = true;
            continue;
        } else if rest.starts_with(close) {
            start += close.len();
            *is_inside_block = false;
            continue;
        } else if !*is_inside_block {
            if is_single_line_comment(rest, &comment_type.line) {
                return !code_present;
            }
            code_present = true;
        }
        start += 1;
    }
    !code_present
}

#[cfg(test)]
mod tests {
    mod count_lines {
        use crate::analysis::count_lines;
        use crate::registry::{Block, CommentType};
        use std::{io::Write, path::Path};
        use tempfile::NamedTempFile;

        #[test]
        fn empty_file() {
            let file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }

        #[test]
        fn not_a_file() {
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            let res = count_lines(Path::new("./"), &comments);
            assert!(res.is_err());
        }

        #[test]
        fn only_comments_and_newlines() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"//
                //
                // text
                //
                
                
                //
                    
                "#
            )
            .unwrap();

            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }
        #[test]
        fn single_line_comments_with_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"//
                // text
                //
                // text
                code
                code
                    code
                code
                //

                "#
            )
            .unwrap();

            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 4);
        }

        #[test]
        fn single_line_comment_after_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"//
                code
                code

                code // text
                //text 
                "#
            )
            .unwrap();

            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 3);
        }

        #[test]
        fn comments_inside_string() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                let s = "/* not a comment */";
                let s = "// not a comment "#
            )
            .unwrap();

            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 2);
        }

        #[test]
        fn block_multi_line_no_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"/* text
                   text
                */
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }

        #[test]
        fn block_multi_line_code_after() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                 /* text
                    text
                 */ code
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_multi_line_code_before() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                 code /* text
                         text
                      */
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_single_line_no_code() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                 /* text */
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }
        #[test]
        fn block_comments_single_line_code_before() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                 code /* text */
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_single_line_code_after() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                 /* text */ code
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_single_line_interleaved() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                 code /* a */ code /* b */ code
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 1);
        }
        #[test]
        fn block_comments_no_end() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"
                /* text
                    text
                 "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 0);
        }

        #[test]
        fn mixed_line_and_block_comments() {
            let mut file = NamedTempFile::new().unwrap();
            let comments = CommentType {
                line: vec!["//".to_string()],
                block: Some(Block {
                    open: "/*".to_string(),
                    close: "*/".to_string(),
                }),
            };
            write!(
                file,
                r#"/* text
                   text
                */

                /* text
                    text
                */ code

                code /* text
                         text
                      */ // text

                /* text */

                // text
                code /* text */

                /* text */ code

                code // /* a */ code /* b */ code
                //
                /* text
                   text
                "#
            )
            .unwrap();
            let res = count_lines(file.path(), &comments);
            assert_eq!(res.unwrap(), 5);
        }

        // TODO:
        // below test cases do not pass yet.
        // They might be handled in the future, however at the moment counter
        // is accurate enough for a rough estimate Heuristics FTW!
        //
        //#[test]
        //fn multi_line_string() {
        //    let mut file = NamedTempFile::new().unwrap();
        //    let comments = CommentType {
        //        line: vec!["//".to_string()],
        //        block: Block {
        //            open: "/*".to_string(),
        //            close: "*/".to_string(),
        //        },
        //    };
        //    write!(
        //        file,
        //        r#"
        //        let s = "line1
        //        /* block comment inside string
        //         * just ingnore it?
        //         * */
        //        line3";
        //        "#
        //    )
        //    .unwrap();
        //    let res = count_lines(file.path(), &comments);
        //    assert_eq!(res.unwrap(), 5);
        //}
        //#[test]
        //fn windows_newlines() {
        //    let mut file = NamedTempFile::new().unwrap();
        //    let comments = CommentType {
        //        line: vec!["//".to_string()],
        //        block: Block {
        //            open: "/*".to_string(),
        //            close: "*/".to_string(),
        //        },
        //    };
        //    write!(file, "\r\ncode\n//text\r\n").unwrap();
        //    let res = count_lines(file.path(), &comments);
        //    assert_eq!(res.unwrap(), 0);
        //}
    }
    mod is_single_line_comment {
        use crate::analysis::is_single_line_comment;

        #[test]
        fn single_line_comment_c_style() {
            let res = is_single_line_comment(
                "// c style comment",
                &["%".to_string(), "#".to_string(), "//".to_string()],
            );
            assert!(res);
        }

        #[test]
        fn single_line_comment_python() {
            let res = is_single_line_comment(
                "# python comment",
                &["%".to_string(), "#".to_string(), "//".to_string()],
            );
            assert!(res);
        }

        #[test]
        fn single_line_comment_latex() {
            let res = is_single_line_comment(
                "% latex comment",
                &["%".to_string(), "#".to_string(), "//".to_string()],
            );
            assert!(res);
        }

        #[test]
        fn single_line_comment_not_a_comment() {
            let res = is_single_line_comment(
                "! this is not a one-line comment",
                &["%".to_string(), "#".to_string(), "//".to_string()],
            );
            assert!(!res);
        }
    }
    mod is_block_comment {
        use crate::analysis::is_block_comment;
        use crate::registry::{Block, CommentType};

        #[test]
        fn block_comment_valid_single_line_c_style_no_code() {
            let mut is_inside_block = false;
            let res = is_block_comment(
                "/* single line */",
                &mut is_inside_block,
                &CommentType {
                    line: vec!["//".to_string()],
                    block: Some(Block {
                        open: "/*".to_string(),
                        close: "*/".to_string(),
                    }),
                },
            );
            assert!(res);
            assert!(!is_inside_block);
        }

        #[test]
        fn block_comment_valid_single_line_c_style_with_code_and_multiline_start() {
            let mut is_inside_block = false;
            let res = is_block_comment(
                "/* single line */ code /* comment */ code /* another",
                &mut is_inside_block,
                &CommentType {
                    line: vec!["//".to_string()],
                    block: Some(Block {
                        open: "/*".to_string(),
                        close: "*/".to_string(),
                    }),
                },
            );
            assert!(!res);
            assert!(is_inside_block);
        }

        #[test]
        fn block_comment_valid_single_line_c_style_with_code_and_multiline_end() {
            let mut is_inside_block = true;
            let res = is_block_comment(
                "still a comment */ code! ",
                &mut is_inside_block,
                &CommentType {
                    line: vec!["//".to_string()],
                    block: Some(Block {
                        open: "/*".to_string(),
                        close: "*/".to_string(),
                    }),
                },
            );
            assert!(!res);
            assert!(!is_inside_block);
        }

        #[test]
        fn block_comment_valid_single_line_c_style_inside_multiline_no_code() {
            let mut is_inside_block = true;
            let res = is_block_comment(
                "// anything here!",
                &mut is_inside_block,
                &CommentType {
                    line: vec!["//".to_string()],
                    block: Some(Block {
                        open: "/*".to_string(),
                        close: "*/".to_string(),
                    }),
                },
            );
            assert!(res);
            assert!(is_inside_block);
        }
    }
}
