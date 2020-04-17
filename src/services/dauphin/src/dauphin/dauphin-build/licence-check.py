#! /usr/bin/env python3

import subprocess, sys, os.path

matching_suffixes = [
    ".rs"
]

antitext = "Licensed under the Apache License"

p = subprocess.run(['git','rev-parse','--show-toplevel'],stdout=subprocess.PIPE)
root = p.stdout.decode("utf8").strip()

def good_filetype(filename):
    for match in matching_suffixes:
        if filename.endswith(match):
            return True
    return False

changed_files = []
p = subprocess.run(['git','diff-index','--cached','HEAD'],stdout=subprocess.PIPE)
for line in p.stdout.decode("utf8").split("\n"):
    sections = line.split("\t")
    if len(sections) < 2:
        continue
    change = sections[0].split(" ")
    if change[4] in "MA" and good_filetype(sections[1]):
        changed_files.append(sections[1])

missing = []
for filename in changed_files:
    filename = os.path.join(root,filename)
    with open(filename, 'r') as f:
        text = f.read()
        if antitext not in text:
            missing.append(filename)

if len(missing) > 0:
    sys.stderr.write("Missing licences in {0}\n".format(", ".join(missing)))
    sys.exit(1)
else:
    sys.exit(0)
