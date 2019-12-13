import urllib

from flask import request, url_for

def make_url(page,**params):
    instance = request.args.get("instance") or request.form.get("instance") or ""
    path = url_for("."+page)
    if params:
        p = { k: v for (k,v) in params.items() if v }
        if len(p):
            path += "?"+ urllib.parse.urlencode(p)
    return path

def page_path():
    instance = request.args.get("instance") or request.form.get("instance") or ""
    return make_url("blackbox_control",instance = instance)
