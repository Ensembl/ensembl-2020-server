#! /usr/bin/env python3

from dotenv import load_dotenv
from flask import Flask, jsonify, request, send_from_directory, render_template, redirect
from flask_cors import CORS
import sys, os, os.path, yaml, logging, logging.config, errno, urllib, datetime, time
from base64 import b64encode

script_dir = os.path.dirname(os.path.realpath(__file__))

"""
TODO

raw on/off
raw writing
raw display
raw graph
no absolute paths
tests
line counts
"""

# Find and maybe create working directory

load_dotenv()
blackbox_dir = os.path.join(
    os.path.abspath(os.environ.get("BLACKBOX_DIR",".")),
    "log"
)

sys.stderr.write("Using {0} as working directory\n".format(blackbox_dir))

try:
    os.makedirs(blackbox_dir)
    sys.stderr.write("created!\n")
except OSError as e:
    if e.errno == errno.EEXIST:
        sys.stderr.write("Already existed. Fine.\n")
    else:
        raise
sys.stderr.write("\n\n")

# 

def stream_path(stream):
    stream = b64encode(stream.encode("utf8"),b"-_").decode("utf8")
    return os.path.join(blackbox_dir,"stream-{0}.txt".format(stream))

def xxx_make_fake_config():
    config = Config()
    config.enable("xxx")
    config.disable("yyy")
    return config

class Config:
    def __init__(self):
        self.streams_seen = set()
        self.streams_enabled = set()

    def enable(self,value):
        self.streams_seen.add(value)
        self.streams_enabled.add(value)

    def disable(self,value):
        self.streams_seen.add(value)
        self.streams_enabled.discard(value)

    def to_json(self):
        return jsonify({
            "config": {
                "enable": list(self.streams_enabled),
            }
        })

    def get_jinja(self):
        streams = {}
        for stream in self.streams_seen:
            tail = "/blackbox/tail?"+ urllib.parse.urlencode({
                "stream": stream
            })
            streams[stream] = {
                "active": stream in self.streams_enabled,
                "filename": os.path.split(stream_path(stream))[-1],
                "tail": tail
            }
        return {
            "streams": streams
        }

class Model:
    def __init__(self):
        self.config = xxx_make_fake_config()
        

    def process_line(self,line):
        print(line)

def create_app(testing=False):
    model = Model()
    app = Flask(__name__,template_folder="../templates")
    app.testing = testing
    CORS(app)

    @app.route('/blackbox')
    def blackbox_control():
        return render_template("index.html", config = model.config.get_jinja())

    @app.route("/blackbox/<path:asset>")
    def blackbox_asset(asset):
        return send_from_directory(os.path.join(script_dir,"../assets"),asset)

    @app.route("/blackbox/tail")
    def blackbox_tail():
        stream = request.args.get("stream")
        path = stream_path(stream)        
        try:
            with open(path) as f:
                data = f.read()
        except OSError as e:
            if e.errno == errno.ENOENT:
                data = "ERROR: No such file"
            else:
                data = "ERROR: {0}".format(str(e))
        return render_template("tail.html", data = data)

    @app.route("/blackbox/mark",methods=["POST"])
    def blackbox_mark():
        stream = request.form["stream"]
        mark = request.form["mark"]
        path = stream_path(stream)        
        with open(path,"a") as f:
            now = datetime.datetime.fromtimestamp(time.time()).strftime("%Y-%m-%d %H:%M:%S")
            f.write("[{0}] MARK: {1}\n".format(now,mark))
        return redirect("/blackbox")

    @app.route("/blackbox/update-config", methods=["POST"])
    def blackbox_config():
        for (key,value) in request.form.items():
            if key == "enable":
                model.config.enable(value)
            elif key == "disable":
                model.config.disable(value)
        return model.config.to_json()

    @app.route("/blackbox/update-config-page", methods=["POST"])
    def blackbox_page_config():
        blackbox_config()
        return redirect("/blackbox")

    @app.route("/blackbox/data", methods=["POST"])
    def blackbox_data():
        incoming = request.get_json(force=True)
        for line in incoming:
            model.process_line(line)
        sys.stderr.write("Got {0} lines\n".format(len(incoming)))
        return model.config.to_json()
    
    return app

if __name__ == "__main__":
   create_app().run(port=4040,host='0.0.0.0')
