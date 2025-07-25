"""
A script used to generate images from .obj, .glb and .stl 3D models to
use with thumbnails.

Package Requirements: 3d-to-image
Author: tk
"""
import os, io, sys
from model_to_image import process_obj, glb_to_image, process_stl

SUPPORTED_EXTS = {".obj", ".glb", ".stl"}

class ProcessModel:
    def __init__(self, model_path) -> None:
        self.model_path = model_path
        self.model_dir = os.path.dirname(model_path)
        self.model_name = os.path.splitext(os.path.basename(model_path))[0]
        self.thumbnails_dir = os.path.join(self.model_dir, "thumbnails")
        os.makedirs(self.thumbnails_dir, exist_ok=True)
        self.output_dir = os.path.join(self.thumbnails_dir, f"{self.model_name}.png")

    def render(self) -> bool:
        """
        Render the selected model (from the class) and generate an image stored in
        the model_dir/thumbnails/model_name.png
        """
        ext = os.path.splitext(self.model_path)[1].lower()
        image_bytes = None
        if ext == ".obj":
            image_bytes = process_obj(self.model_path)
        elif ext == ".glb":
            with open(self.model_path, "rb") as f:
                glb_bytes = f.read()
            image_bytes = glb_to_image(glb_bytes)
        elif ext == ".stl":
            stl_images = process_stl(self.model_path)
            if "error" in stl_images:
                print(stl_images["error"])
                return False
            if "front" in stl_images and stl_images["front"]:
                image_bytes = stl_images["front"]
            else:
                for img_bytes in stl_images.values():
                    if img_bytes:
                        image_bytes = img_bytes
                        break
        else:
            print(f"Unsupported model format: {ext}")
            return False

        if isinstance(image_bytes, io.BytesIO):
            with open(self.output_dir, "wb") as f:
                f.write(image_bytes.getvalue())
            print(f"Thumbnail saved to {self.output_dir}")
            return True
        else:
            print(f"Model conversion failed or returned error: {image_bytes}")
            return False

def process_path(path):
    if os.path.isdir(path):
        print(f"Processing folder: {path}")
        for fname in os.listdir(path):
            fpath = os.path.join(path, fname)
            if os.path.isfile(fpath) and os.path.splitext(fname)[1].lower() in SUPPORTED_EXTS:
                print(f"Processing file: {fpath}")
                ProcessModel(fpath).render()
    elif os.path.isfile(path):
        ext = os.path.splitext(path)[1].lower()
        if ext in SUPPORTED_EXTS:
            print(f"Processing file: {path}")
            ProcessModel(path).render()
        else:
            print(f"Unsupported file format: {ext}")
    else:
        print(f"Path does not exist: {path}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python convert_model_to_image.py <file_or_folder_path>")
        return
    process_path(sys.argv[1])

if __name__ == "__main__":
    main()
