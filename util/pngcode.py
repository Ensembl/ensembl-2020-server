#! /usr/bin/env python3

import sys
import png
import base64

image = png.Reader(filename=sys.argv[1])
data = []
for row in image.asRGBA8()[2]:
    data += row

print(base64.b64encode(bytearray(data)))
