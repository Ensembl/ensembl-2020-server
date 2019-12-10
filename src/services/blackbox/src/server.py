#! /usr/bin/env python3

from dotenv import load_dotenv
from flask import Flask, jsonify, request, send_from_directory, render_template, redirect, Response
from flask_cors import CORS
import sys, os, os.path, yaml, logging, logging.config, errno, urllib, datetime, time, collections,json
from base64 import b64encode

script_dir = os.path.dirname(os.path.realpath(__file__))

"""
TODO

raw -> dataset
ORM
time in raw data
time in all summaries
time in log writes
formatted log writes
truncation
raw graph
no absolute paths
tests
line counts
"""

EXAMPLE_DATA = """
[
            {"instance":"test1","stack":["a","b"],"text":"Hello, world!","time":2.0},
            {
                "data":[2.0],"dataset":"raw","instance":"test1",
                "count": 1, "total": 2.0, "mean": 2.0, "high": 2.0, "top": 2.0,
                "text":"raw elapsed: num=1 total=2.00units avg=2.00units 95%ile=2.00units top=2.00units","time":2.0
            }
]
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
    stream = b64encode(stream.encode("utf8"),b"-_").decode("utf8").rstrip("=")
    return os.path.join(blackbox_dir,"stream-{0}.txt".format(stream))

def dataset_path(stream,raw):
    stream = b64encode(stream.encode("utf8"),b"-_").decode("utf8").rstrip("=")
    raw = b64encode(raw.encode("utf8"),b"-_").decode("utf8").rstrip("=")
    return os.path.join(blackbox_dir,"dataset-{0}-{1}.txt".format(stream,raw))

def rawdata_path(stream,raw):
    stream = b64encode(stream.encode("utf8"),b"-_").decode("utf8").rstrip("=")
    raw = b64encode(raw.encode("utf8"),b"-_").decode("utf8").rstrip("=")
    return os.path.join(blackbox_dir,"rawdata-{0}-{1}.txt".format(stream,raw))

def xxx_make_fake_config():
    config = Config()
    config.enable("test1")
    config.disable("test2")
    config.enable_raw("test1","raw")
    config.disable_raw("test1","noraw")
    return config

class Config:
    def __init__(self):
        self.streams_seen = set()
        self.streams_enabled = set()
        self.raw_seen = set()
        self.raw_enabled = set()

    def seen(self,value):
        self.streams_seen.add(value)

    def seen_raw(self,stream,value):
        self.raw_seen.add((stream,value))

    def enable(self,value):
        self.streams_seen.add(value)
        self.streams_enabled.add(value)

    def disable(self,value):
        self.streams_seen.add(value)
        self.streams_enabled.discard(value)

    def enable_raw(self,stream,value):
        self.streams_seen.add(stream)
        self.raw_seen.add((stream,value))
        self.raw_enabled.add((stream,value))

    def disable_raw(self,stream,value):
        self.streams_seen.add(stream)
        self.raw_seen.add((stream,value))
        self.raw_enabled.discard((stream,value))

    def to_json(self):
        return jsonify({
            "config": {
                "enable": list(self.streams_enabled),
            }
        })

    def get_jinja(self):
        streams = {}
        raws = {}
        for stream in self.streams_seen:
            tail = "/blackbox/tail?"+ urllib.parse.urlencode({
                "stream": stream
            })
            streams[stream] = {
                "active": stream in self.streams_enabled,
                "filename": os.path.split(stream_path(stream))[-1],
                "tail": tail,
            }
        for (stream,raw) in self.raw_seen:
            summary_url = "/blackbox/dataset?"+ urllib.parse.urlencode({
                "stream": stream,
                "dataset": raw
            })
            rawdata_url = "/blackbox/rawdata?"+ urllib.parse.urlencode({
                "stream": stream,
                "dataset": raw
            })
            raws[(stream,raw)] = {
                "enabled": (stream,raw) in self.raw_enabled,
                "filename": os.path.split(dataset_path(stream,raw))[-1],
                "raw_filename": os.path.split(rawdata_path(stream,raw))[-1],
                "summary_url": summary_url,
                "rawdata_url": rawdata_url
            }
        return {
            "streams": streams,
            "raws": raws
        }

class Model:
    def __init__(self):
        self.config = Config()
        # XXX
        incoming = json.loads(EXAMPLE_DATA)
        for line in incoming:
            self.process_line(line)

    def write_line(self,line):
        with open(stream_path(line["instance"]),"a") as f:
            f.write("{0}\n".format(line["text"]))

    def write_dataset(self,line):
        with open(dataset_path(line["instance"],line["dataset"]),"a") as f:
            f.write("{0}\n".format(json.dumps(line).replace("\n"," ")))

    def write_data(self,line):
        with open(rawdata_path(line["instance"],line["dataset"]),"a") as f:
            for point in line["data"]:
                f.write("{0}\n".format(point))

    def process_line(self,line):
        self.config.seen(line["instance"])
        self.write_line(line)
        if "dataset" in line:
            self.config.seen_raw(line["instance"],line["dataset"])
            self.write_dataset(line)
            if "data" in line:
                self.write_data(line)
        print(line)

    def get_jinja(self):
        datasets = {}
        for (stream,raw) in self.config.raw_seen:
            try:
                with open(dataset_path(stream,raw)) as f:
                    lines = f.readlines()
                    if len(lines) > 0:
                        data = json.loads(lines[-1])
                        datasets[(stream,raw)] = data
            except OSError as e:
                if e.errno == errno.ENOENT:
                    pass
        return {
            "datasets": datasets
        }

def create_app(testing=False):
    model = Model()
    app = Flask(__name__,template_folder="../templates")
    app.testing = testing
    CORS(app)

    @app.route('/blackbox')
    def blackbox_control():
        return render_template("index.html", config = model.config.get_jinja(), data = model.get_jinja())

    @app.route("/blackbox/<path:asset>")
    def blackbox_asset(asset):
        return send_from_directory(os.path.join(script_dir,"../assets"),asset)

    @app.route("/blackbox/dataset")
    def blackbox_dataset():
        stream = request.args.get("stream")
        dataset = request.args.get("dataset")
        columns = ["count","total","mean","high","top"]
        names = { k: k for k in columns }
        names["high"] = "95%ile"
        data = "\t".join([ names[x] for x in columns ]) + "\n"
        try:
            with open(dataset_path(stream,dataset)) as f:
                for line in f.readlines():
                    row = json.loads(line)
                    data += "\t".join([ str(row[x]) for x in columns ]) + "\n"
        except OSError as e:
            if e.errno == errno.ENOENT:
                pass
        return Response(data,mimetype="text/plain")

    @app.route("/blackbox/rawdata")
    def blackbox_raw():
        stream = request.args.get("stream")
        dataset = request.args.get("dataset")
        data = ""
        try:
            with open(rawdata_path(stream,dataset)) as f:
                data = f.read()
        except OSError as e:
            if e.errno == errno.ENOENT:
                pass
        return Response(data,mimetype="text/plain")

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
            elif key == "raw-enable":
                stream = request.form["raw-stream"]
                model.config.enable_raw(stream,value)
            elif key == "raw-disable":
                stream = request.form["raw-stream"]
                model.config.disable_raw(stream,value)
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
