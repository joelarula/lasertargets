import json
import os

COLOR_MAP = {
    0: (0, 255, 255),    # Cyan / Default laser
    1: (255, 0, 0),      # Red
    2: (0, 255, 0),      # Green
    3: (255, 255, 0),    # Yellow
    4: (0, 0, 255),      # Blue
    5: (255, 0, 255),    # Magenta
    6: (255, 255, 255),  # White
    7: (0, 0, 0),        # Blanking / Off
}

def convert_point(pt):
    raw_x = float(pt[0])
    raw_y = float(pt[1])
    
    # Scale from [-400, 400] laser bounds to [-1.0, 1.0] normalized template space
    x = round(raw_x / 400.0, 4) if abs(raw_x) > 1.0 else round(raw_x, 4)
    y = round(raw_y / 400.0, 4) if abs(raw_y) > 1.0 else round(raw_y, 4)
    
    color_code = int(pt[2]) if len(pt) > 2 else 0
    dwell = int(pt[3]) if len(pt) > 3 else 0
    
    r, g, b = COLOR_MAP.get(color_code, (255, 255, 255))
    
    return {
        "x": x,
        "y": y,
        "r": r,
        "g": g,
        "b": b,
        "dwell": dwell
    }

def convert_file(src_path, output_dir, prefix, tag_name):
    if not os.path.exists(src_path):
        print(f"File not found: {src_path}")
        return 0

    with open(src_path, 'r') as f:
        data = json.load(f)

    os.makedirs(output_dir, exist_ok=True)
    count = 0

    for idx, shape_pts in enumerate(data, start=1):
        if not isinstance(shape_pts, list):
            continue
            
        template_name = f"{prefix}_{idx:03d}"
        points = [convert_point(pt) for pt in shape_pts if isinstance(pt, list) and len(pt) >= 2]
        
        template = {
            "name": template_name,
            "description": f"Draft template #{idx} converted from {os.path.basename(src_path)}",
            "tags": ["draft", tag_name],
            "author": "DraftConverter",
            "line_style": "Continuous",
            "points": points
        }

        dest_file = os.path.join(output_dir, f"{template_name}.json")
        with open(dest_file, 'w') as out_f:
            json.dump(template, out_f, indent=2)
        count += 1

    print(f"Converted {count} shapes from {os.path.basename(src_path)} -> {output_dir}")
    return count

base_dir = r"c:\Users\joela\dev\lasertargets\assets\shapes"
templates_draft_dir = os.path.join(base_dir, "templates", "draft")

c1 = convert_file(os.path.join(base_dir, "lineShapes.json"), os.path.join(templates_draft_dir, "lineshapes"), "draft_lineshape", "lineshape")
c2 = convert_file(os.path.join(base_dir, "picArrayShapes.json"), os.path.join(templates_draft_dir, "picarray"), "draft_picarray", "picarray")
c3 = convert_file(os.path.join(base_dir, "shapePatternTemplates.json"), os.path.join(templates_draft_dir, "pattern"), "draft_pattern", "pattern")

print(f"Total draft templates created: {c1 + c2 + c3}")
