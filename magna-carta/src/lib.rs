pub mod generator;

use std::path::PathBuf;
use tree_sitter::{Parser, Query, QueryCursor};

#[derive(Debug, Clone)]
pub struct ScriptManifest {
    items: Vec<ManifestItem>,
}

impl Default for ScriptManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptManifest {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_item(&mut self, item: ManifestItem) {
        self.items.push(item);
    }

    pub fn items(&self) -> &[ManifestItem] {
        &self.items
    }
}

#[derive(Debug, Clone)]
pub struct ManifestItem {
    /// Fully qualified class name
    ///
    /// Example: `foo.bar.Enemy`
    fqcn: String,
    /// Simple name of the class
    ///
    /// Example: `Enemy`
    simple_name: String,
    /// Tags to identify the class
    ///
    /// Example: `["goomba", "shell"]`
    tags: Vec<String>,
    /// Path to the source file in reference to the project root
    ///
    /// Example: `src/commonMain/kotlin/foo/bar/Enemy.kt`
    file_path: PathBuf,
}

impl ManifestItem {
    pub fn new(fqcn: String, simple_name: String, tags: Vec<String>, file_path: PathBuf) -> Self {
        Self {
            fqcn,
            simple_name,
            tags,
            file_path,
        }
    }

    pub fn fqcn(&self) -> &str {
        &self.fqcn
    }

    pub fn simple_name(&self) -> &str {
        &self.simple_name
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }
}

pub struct KotlinProcessor {
    parser: Parser,
}

impl KotlinProcessor {
    pub fn new() -> anyhow::Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_kotlin::language())?;
        Ok(Self { parser })
    }

    pub fn process_file(
        &mut self,
        source_code: &str,
        file_path: PathBuf,
    ) -> anyhow::Result<Option<ManifestItem>> {
        let tree = self
            .parser
            .parse(source_code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse source code"))?;

        let root_node = tree.root_node();

        let package = self.extract_package(root_node, source_code)?;

        if let Some(class_info) = self.extract_class_info(root_node, source_code)? {
            let (class_name, tags) = class_info;

            let fqcn = if package.is_empty() {
                class_name.clone()
            } else {
                format!("{}.{}", package, class_name)
            };

            return Ok(Some(ManifestItem::new(fqcn, class_name, tags, file_path)));
        }

        Ok(None)
    }

    fn extract_package(
        &self,
        root_node: tree_sitter::Node,
        source: &str,
    ) -> anyhow::Result<String> {
        let query = Query::new(
            &tree_sitter_kotlin::language(),
            r#"
            (package_header
              (identifier) @package)
            "#,
        )?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root_node, source.as_bytes());

        if let Some(match_) = matches.next()
            && let Some(capture) = match_.captures.first()
        {
            let package_node = capture.node;
            let package_text = package_node.utf8_text(source.as_bytes())?;

            return Ok(package_text.replace('\n', "").trim().to_string());
        }

        Ok(String::new())
    }

    fn extract_class_info(
        &self,
        root_node: tree_sitter::Node,
        source: &str,
    ) -> anyhow::Result<Option<(String, Vec<String>)>> {
        let query = Query::new(
            &tree_sitter_kotlin::language(),
            r#"
        ; Case 1: @Runnable (no parentheses)
        (class_declaration
          (modifiers
            (annotation
              (user_type
                (type_identifier) @annotation_name)
              (#eq? @annotation_name "Runnable")))
          (type_identifier) @class_name)

        ; Case 2: @Runnable(...) (with parentheses)
        (class_declaration
          (modifiers
            (annotation
              (constructor_invocation
                (user_type
                  (type_identifier) @annotation_name2)
                (value_arguments)? @value_args)
              (#eq? @annotation_name2 "Runnable")))
          (type_identifier) @class_name2)
        "#,
        )?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, root_node, source.as_bytes());

        let annotation_name_idx = query.capture_index_for_name("annotation_name");
        let annotation_name2_idx = query.capture_index_for_name("annotation_name2");
        let class_name_idx = query.capture_index_for_name("class_name");
        let class_name2_idx = query.capture_index_for_name("class_name2");
        let value_args_idx = query.capture_index_for_name("value_args");

        for match_ in matches {
            let mut class_name = String::new();
            let mut found_runnable = false;
            let mut value_args_node = None;

            for capture in match_.captures {
                let text = capture.node.utf8_text(source.as_bytes())?;

                // case 1 (no brackets)
                if let Some(idx) = annotation_name_idx
                    && capture.index == idx
                    && text == "Runnable"
                {
                    found_runnable = true;
                }

                // case 2 (with brackets)
                if let Some(idx) = annotation_name2_idx
                    && capture.index == idx
                    && text == "Runnable"
                {
                    found_runnable = true;
                }

                // class names
                if let Some(idx) = class_name_idx
                    && capture.index == idx
                {
                    class_name = text.to_string();
                }

                if let Some(idx) = class_name2_idx
                    && capture.index == idx
                {
                    class_name = text.to_string();
                }

                // case 2 value args
                if let Some(idx) = value_args_idx
                    && capture.index == idx
                {
                    value_args_node = Some(capture.node);
                }
            }

            if found_runnable && !class_name.is_empty() {
                let tags = if let Some(value_args) = value_args_node {
                    self.extract_tags_from_value_args(value_args, source)?
                } else {
                    Vec::new()
                };

                return Ok(Some((class_name, tags)));
            }
        }

        Ok(None)
    }

    fn extract_tags_from_value_args(
        &self,
        value_args_node: tree_sitter::Node,
        source: &str,
    ) -> anyhow::Result<Vec<String>> {
        let mut tags = Vec::new();

        let mut cursor = value_args_node.walk();
        for value_arg in value_args_node.children(&mut cursor) {
            if value_arg.kind() == "value_argument" {
                let mut arg_cursor = value_arg.walk();
                for child in value_arg.children(&mut arg_cursor) {
                    // Case 1: Direct string literal (vararg style)
                    if child.kind() == "string_literal" {
                        let text = child.utf8_text(source.as_bytes())?;
                        let clean_tag = text.trim_matches(|c| c == '"' || c == '\'').to_string();
                        if !clean_tag.is_empty() {
                            tags.push(clean_tag);
                        }
                    } else if child.kind() == "collection_literal" {
                        let mut collection_cursor = child.walk();
                        for collection_item in child.children(&mut collection_cursor) {
                            if collection_item.kind() == "string_literal" {
                                let text = collection_item.utf8_text(source.as_bytes())?;
                                let clean_tag =
                                    text.trim_matches(|c| c == '"' || c == '\'').to_string();
                                if !clean_tag.is_empty() {
                                    tags.push(clean_tag);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_player_class() {
        let source = r#"
package com.dropbear

@Runnable(["player", "movement"])
class Player: System {
    override fun load(engine: DropbearEngine) {
        TODO("Not yet implemented")
    }
    override fun update(engine: DropbearEngine, deltaTime: Float) {

        TODO("Not yet implemented")

    }

    override fun destroy(engine: DropbearEngine) {

        TODO("Not yet implemented")

    }

}
"#;

        let mut processor = KotlinProcessor::new().unwrap();
        let result = processor
            .process_file(
                source,
                PathBuf::from("src/main/kotlin/com/dropbear/Player.kt"),
            )
            .unwrap();

        assert!(result.is_some());
        let item = result.unwrap();

        assert_eq!(item.fqcn(), "com.dropbear.Player");
        assert_eq!(item.simple_name(), "Player");
        assert_eq!(item.tags(), &["player", "movement"]);
        assert_eq!(
            item.file_path(),
            &PathBuf::from("src/main/kotlin/com/dropbear/Player.kt")
        );
    }
}
