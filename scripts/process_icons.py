import os
from PIL import Image

def process_icons():
    source_image = "NEW-updated.png"
    if not os.path.exists(source_image):
        print(f"Error: {source_image} not found.")
        return

    try:
        img = Image.open(source_image)
        
        # Save as ICO
        # ICO usually contains multiple sizes: 16, 32, 48, 64, 128, 256
        icon_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
        img.save("app_icon.ico", format="ICO", sizes=icon_sizes)
        print("Generated app_icon.ico")

        # Save a resized PNG for the app usage (e.g. 128x128 or 256x256) to avoid loading 5MB
        # Let's save a 256x256 version for window icon usage if needed, 
        # though we can load the ICO or the original PNG. 
        # But loading 5MB PNG at startup is waste.
        img_resized = img.resize((256, 256), Image.Resampling.LANCZOS)
        img_resized.save("icon_256.png")
        print("Generated icon_256.png")
        
    except Exception as e:
        print(f"Error processing image: {e}")

if __name__ == "__main__":
    process_icons()
