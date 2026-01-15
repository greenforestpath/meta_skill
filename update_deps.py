import re
import requests
import sys

def get_latest_version(crate_name):
    url = f"https://crates.io/api/v1/crates/{crate_name}"
    try:
        response = requests.get(url, headers={'User-Agent': 'ms-updater'}, timeout=5)
        if response.status_code == 200:
            data = response.json()
            return data['crate']['max_version']
    except Exception as e:
        print(f"Error fetching {crate_name}: {e}", file=sys.stderr)
    return None

def update_line(line):
    # Regex to capture: name = { version = "..." ... } or name = "..."
    # Simplified: look for package name at start of line, then version
    
    match = re.match(r'^([a-z0-9-_]+)\s*=\s*(.*)', line.strip())
    if not match:
        return line
        
    name = match.group(1)
    rest = match.group(2)
    
    # Skip section headers or other keys that aren't dependencies (approximate)
    # We will assume we are processing dependencies because we'll only run this on relevant lines? 
    # No, it's hard to parse TOML without a parser.
    
    # Better approach: Iterate lines. If we are in a dependency section, try to update.
    
    return line

def process_file():
    with open("Cargo.toml", "r") as f:
        lines = f.readlines()

    in_deps = False
    new_lines = []
    
    dep_sections = ['[dependencies]', '[dev-dependencies]', '[build-dependencies]']
    
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('[') and stripped.endswith(']'):
            in_deps = any(stripped.startswith(s) for s in dep_sections)
            new_lines.append(line)
            continue
            
        if not in_deps or stripped.startswith('#') or not stripped:
            new_lines.append(line)
            continue
            
        # Attempt to parse dependency
        # name = "version"
        # name = { version = "version", ... }
        
        # Check for inline table
        match_inline = re.match(r'^([a-z0-9-_]+)\s*=\s*\{\s*version\s*=\s*"([^"]+)"(.*)\}', stripped)
        if match_inline:
            name = match_inline.group(1)
            current_ver = match_inline.group(2)
            suffix = match_inline.group(3)
            latest = get_latest_version(name)
            if latest and latest != current_ver:
                print(f"Updating {name}: {current_ver} -> {latest}")
                line = f'{name} = {{ version = "{latest}"{suffix}}}\n'
            new_lines.append(line)
            continue

        # Check for simple string
        match_str = re.match(r'^([a-z0-9-_]+)\s*=\s*"([^"]+)"', stripped)
        if match_str:
            name = match_str.group(1)
            current_ver = match_str.group(2)
            latest = get_latest_version(name)
            if latest and latest != current_ver:
                print(f"Updating {name}: {current_ver} -> {latest}")
                line = f'{name} = "{latest}"\n'
            new_lines.append(line)
            continue
            
        new_lines.append(line)

    with open("Cargo.toml", "w") as f:
        f.writelines(new_lines)

process_file()
