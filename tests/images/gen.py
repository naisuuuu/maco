#!/usr/bin/python

from PIL import Image, ImageOps

with Image.open("wikipe-tan.png") as im:
    im = ImageOps.grayscale(im)
    im.save("wikipe-tan-grayscale.png", "PNG")
    im = ImageOps.autocontrast(im, cutoff=1, preserve_tone=True)
    im = Image.eval(im, lambda a: ((a / 255.) ** (1 / 0.75)) * 255)
    im.save("wikipe-tan-want.png", "PNG")
