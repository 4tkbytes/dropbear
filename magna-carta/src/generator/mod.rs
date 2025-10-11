use crate::ScriptManifest;
use std::path::Path;

pub mod jvm;
pub mod native;

#[allow(dead_code)]
pub trait Generator {
    fn generate(&self, manifest: &ScriptManifest) -> anyhow::Result<String>;

    fn write_to_file(
        &self,
        manifest: &ScriptManifest,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let content = self.generate(manifest)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ManifestItem;
    use crate::generator::jvm::KotlinJVMGenerator;
    use crate::generator::native::KotlinNativeGenerator;
    use std::path::PathBuf;

    #[test]
    fn test_native_generator() {
        let mut manifest = ScriptManifest::new();
        manifest.add_item(ManifestItem::new(
            "com.game.Player".to_string(),
            "Player".to_string(),
            vec!["player".to_string(), "movement".to_string()],
            PathBuf::from("src/Player.kt"),
        ));

        manifest.add_item(ManifestItem::new(
            "com.game.GlobalLogger".to_string(),
            "GlobalLogger".to_string(),
            vec![],
            PathBuf::from("src/GlobalLogger.kt"),
        ));

        let generator = KotlinNativeGenerator;
        let output = generator.generate(&manifest).unwrap();

        assert!(output.contains("import com.game.Player"));
        assert!(output.contains("import com.game.GlobalLogger"));

        assert!(output.contains("tags = listOf(\"player\", \"movement\")"));
        assert!(output.contains("tags = listOf()"));

        assert!(output.contains("script = Player()"));
        assert!(output.contains("script = GlobalLogger()"));

        assert!(output.contains("@CName(\"dropbear_load\")"));
        assert!(output.contains("@CName(\"dropbear_update\")"));
        assert!(output.contains("@CName(\"dropbear_destroy\")"));
    }

    #[test]
    fn test_jvm_generator() {
        let mut manifest = ScriptManifest::new();
        manifest.add_item(ManifestItem::new(
            "com.game.Player".to_string(),
            "Player".to_string(),
            vec!["player".to_string()],
            PathBuf::from("src/Player.kt"),
        ));

        let generator = KotlinJVMGenerator;
        let output = generator.generate(&manifest).unwrap();

        assert!(output.contains("import com.game.*"));
        assert!(output.contains("Player::class"));
    }
}
