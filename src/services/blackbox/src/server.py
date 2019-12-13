#! /usr/bin/env python3

from dotenv import load_dotenv
from flask import Flask, jsonify, request, send_from_directory, render_template, redirect, Response, Blueprint, url_for
from flask_cors import CORS
import sys, os, os.path, yaml, logging, logging.config, errno, urllib, datetime, time, collections,json, re
from base64 import b64encode

script_dir = os.path.dirname(os.path.realpath(__file__))

"""
TODO

ORM
CSS
OpenAPI
tests
README

"""

EXAMPLE_DATA = """
[
            {"stream":"test1","stack":["a","b"],"text":"Hello, world!","time":2.0,"instance": "abc"},
            {
                "data":[2.0],"dataset":"raw","stream":"test1","ago":[0.0],"instance": "def",
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

def fmtime(when):
    return datetime.datetime.fromtimestamp(when).strftime("%Y-%m-%d %H:%M:%S")

# 

def fscode(text):
    text = urllib.parse.quote(text,safe='')
    text = text.replace('%','@')
    return text

def stream_path(stream):
    stream = fscode(stream)
    return os.path.join(blackbox_dir,"stream_{0}.txt".format(stream))

def dataset_path(stream,dataset):
    stream = fscode(stream)
    dataset = fscode(dataset)
    return os.path.join(blackbox_dir,"dataset_{0}_{1}.txt".format(stream,dataset))

def rawdata_path(stream,dataset):
    stream = fscode(stream)
    dataset = fscode(dataset)
    return os.path.join(blackbox_dir,"rawdata_{0}_{1}.txt".format(stream,dataset))

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
        self.dataset_seen = set()
        self.raw_enabled = set()

    def seen(self,value):
        self.streams_seen.add(value)

    def seen_dataset(self,stream,value):
        self.dataset_seen.add((stream,value))

    def enable(self,value):
        self.streams_seen.add(value)
        self.streams_enabled.add(value)

    def disable(self,value):
        self.streams_seen.add(value)
        self.streams_enabled.discard(value)

    def enable_raw(self,stream,value):
        self.streams_seen.add(stream)
        self.dataset_seen.add((stream,value))
        self.raw_enabled.add((stream,value))

    def disable_raw(self,stream,value):
        self.streams_seen.add(stream)
        self.dataset_seen.add((stream,value))
        self.raw_enabled.discard((stream,value))

    def to_json(self):
        return jsonify({
            "config": {
                "enable": list(self.streams_enabled),
            }
        })

    def get_jinja(self):
        streams = {}
        datasets = {}
        for stream in self.streams_seen:
            tail = make_url('blackbox_tail', 
                stream = stream,
                instance = request.args.get("instance") or ""
            )
            loglines = "-"
            try:
                with open(stream_path(stream)) as f:
                    loglines = str(len(f.readlines()))
            except OSError as e:
                if e.errno == errno.ENOENT:
                    pass
            streams[stream] = {
                "active": stream in self.streams_enabled,
                "filename": os.path.split(stream_path(stream))[-1],
                "lines": loglines,
                "tail": tail,
            }
        for (stream,dataset) in self.dataset_seen:
            summary_url = make_url('blackbox_dataset',
                stream = stream,
                dataset = dataset,
                instance = request.args.get("instance") or ""
            )
            rawdata_url = make_url('blackbox_rawdata',
                stream = stream,
                dataset = dataset,
                instance = request.args.get("instance") or ""
            )
            datasets[(stream,dataset)] = {
                "enabled": (stream,dataset) in self.raw_enabled,
                "filename": os.path.split(dataset_path(stream,dataset))[-1],
                "dataset_filename": os.path.split(rawdata_path(stream,dataset))[-1],
                "summary_url": summary_url,
                "rawdata_url": rawdata_url
            }
        return {
            "streams": streams,
            "datasets": datasets
        }

class Model:
    def __init__(self):
        self.config = Config()
        # XXX
        incoming = json.loads(EXAMPLE_DATA)
        for record in incoming:
            record["time"] = time.time()
        for line in incoming:
            self.process_line(line)

    def truncate(self,stream):
        doomed = []
        stream = fscode(stream)
        prefixes = [ x.format(stream) for x in ("dataset_{0}_","stream_{0}.","rawdata_{0}_")]
        for filename in os.listdir(blackbox_dir):
            parts = filename.split("-")
            for prefix in prefixes:
                if filename.startswith(prefix):
                    doomed.append(filename)
        for filename in doomed:
            with open(os.path.join(blackbox_dir,filename),"w") as f:
                pass
        return doomed

    def write_line(self,line):
        with open(stream_path(line["stream"]),"a") as f:
            text = line["text"]
            if "stack" in line and len(line["stack"]) > 0:
                text = "({0}) {1}".format("/".join(line["stack"]),text)
            f.write("[{0}] ({1}) {2}\n".format(fmtime(line['time']),line['instance'],text))

    def write_dataset(self,line):
        filename = dataset_path(line["stream"],line["dataset"])
        columns = ["time","instance","count","total","mean","high","top"]
        data = ""
        if (not os.path.exists(filename)) or os.path.getsize(filename) == 0:
            with open(dataset_path(line["stream"],line["dataset"]),"w") as f:
                names = { k: k for k in columns }
                names["high"] = "95%ile"
                data += "\t".join([ names[x] for x in columns ]) + "\n"
        with open(filename,"a") as f:
            data += "\t".join([ str(line[x]) for x in columns ]) + "\n"
            f.write(data)

    def write_data(self,line):
        with open(rawdata_path(line["stream"],line["dataset"]),"a") as f:
            for (ago,point) in zip(line["ago"],line["data"]):
                f.write("{0}\t{1}\t{2}\n".format(line["time"]-ago,line["instance"],point))

    def process_line(self,line):
        self.config.seen(line["stream"])
        self.write_line(line)
        if "dataset" in line:
            self.config.seen_dataset(line["stream"],line["dataset"])
            self.write_dataset(line)
            if "data" in line:
                self.write_data(line)
        print(line)

instance_re = re.compile(r'\[.*?\] \((.*?)\)')

def blackbox():
    model = Model()
    bb = Blueprint('blackbox', __name__,template_folder='../templates')


    @bb.route('/')
    def blackbox_control():
        instance = request.args.get("instance") or ""
        return render_template("index.html", config = model.config.get_jinja(),instance = instance)

    @bb.route("/<path:asset>")
    def blackbox_asset(asset):
        return send_from_directory(os.path.join(script_dir,"../assets"),asset)

    @bb.route("/dataset")
    def blackbox_dataset():
        stream = request.args.get("stream")
        dataset = request.args.get("dataset")
        instance = request.args.get("instance")
        data = ""
        try:
            with open(dataset_path(stream,dataset)) as f:
                headings = f.readline().split("\t")
                data += "\t".join(headings)
                try:
                    instance_heading = headings.index("instance")
                    for line in f.readlines():
                        if instance:
                            parts = line.split("\t")
                            if parts[instance_heading] != instance:
                                continue
                        data += line
                except ValueError:
                    pass
        except OSError as e:
            if e.errno == errno.ENOENT:
                pass
        return Response(data,mimetype="text/plain")

    @bb.route("/rawdata")
    def blackbox_rawdata():
        stream = request.args.get("stream")
        dataset = request.args.get("dataset")
        instance = request.args.get("instance")
        data = ""
        try:
            with open(rawdata_path(stream,dataset)) as f:
                for line in f.readlines():
                    if instance:
                        parts = line.split("\t")
                        if parts[1] != instance:
                            continue
                    data += line
        except OSError as e:
            if e.errno == errno.ENOENT:
                pass
        return Response(data,mimetype="text/plain")

    @bb.route("/tail")
    def blackbox_tail():
        stream = request.args.get("stream")
        instance = request.args.get("instance") or ""
        path = stream_path(stream)        
        out = ""
        try:
            with open(path) as f:
                for line in f.readlines():
                    if instance:
                        m = instance_re.match(line)
                        if m and m.group(1) != instance:
                            continue
                    out += line
        except OSError as e:
            if e.errno == errno.ENOENT:
                data = "ERROR: No such file"
            else:
                data = "ERROR: {0}".format(str(e))
        return render_template("tail.html", data = out)

    @bb.route("/mark",methods=["POST"])
    def blackbox_mark():
        stream = request.form["stream"]
        mark = request.form["mark"]
        path = stream_path(stream)        
        with open(path,"a") as f:
            f.write("[{0}] MARK: {1}\n".format(fmtime(time.time()),mark))
        return redirect(page_path())

    @bb.route("/update-config", methods=["POST"])
    def blackbox_config():
        for (key,value) in request.form.items():
            if key == "enable":
                model.config.enable(value)
            elif key == "disable":
                model.config.disable(value)
            elif key == "raw-enable":
                stream = request.form["stream"]
                model.config.enable_raw(stream,value)
            elif key == "raw-disable":
                stream = request.form["stream"]
                model.config.disable_raw(stream,value)
        return model.config.to_json()

    @bb.route("/update-config-page", methods=["POST"])
    def blackbox_page_config():
        blackbox_config()
        return redirect(page_path())

    @bb.route("/truncate", methods=["POST"])
    def blackbox_truncate():
        stream = request.form["stream"]
        model.truncate(stream)
        return redirect(page_path())

    @bb.route("/data", methods=["POST"])
    def blackbox_data():
        incoming = request.get_json(force=True)
        for line in incoming:
            model.process_line(line)
        return model.config.to_json()

    return bb

if __name__ == "__main__":
    app = Flask(__name__)
    app.register_blueprint(blackbox(),url_prefix="/blackbox")
    CORS(app)
    app.run(port=4040,host='0.0.0.0')
