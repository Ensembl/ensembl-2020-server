import os.path, json, errno

from flask import jsonify, request
from util import make_url, page_path

class Config:
    def __init__(self,blackbox_dir):
        self.filename = os.path.join(blackbox_dir,"config.json")
        self.streams_seen = set()
        self.streams_enabled = set()
        self.dataset_seen = set()
        self.raw_enabled = set()
        self.load()

    def load(self):
        if os.path.exists(self.filename):
            with open(self.filename) as f:
                data = json.load(f)
                self.streams_seen = set(data['streams_seen'])
                self.streams_enabled = set(data['streams_enabled'])
                self.dataset_seen = set([tuple(x) for x in data['dataset_seen']])
                self.raw_enabled = set([tuple(x) for x in data['raw_enabled']])

    def save(self):
        with open(self.filename,"w") as f:
            json.dump({
                'streams_seen': list(self.streams_seen),
                'streams_enabled': list(self.streams_enabled),
                'dataset_seen': list(self.dataset_seen),
                'raw_enabled': list(self.raw_enabled)
            },f)
            
    def seen(self,value):
        self.streams_seen.add(value)

    def delete_stream(self,stream):
        self.streams_seen.discard(stream)
        self.streams_enabled.discard(stream)
        self.dataset_seen = set([x for x in self.dataset_seen if x[0] != stream ])
        self.raw_enabled = set([x for x in self.raw_enabled if x[0] != stream ])

    def delete_dataset(self,stream,dataset):
        self.dataset_seen = set([x for x in self.dataset_seen if x != (stream,dataset) ])
        self.raw_enabled = set([x for x in self.raw_enabled if x != (stream,dataset) ])

    def seen_dataset(self,stream,value):
        self.streams_seen.add(stream)
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
        raw = {}
        for stream in self.streams_seen:
            raw[stream] = [ raw for (rstream,raw) in self.raw_enabled if rstream == stream ]
        return jsonify({
            "config": {
                "enable": list(self.streams_enabled),
                "raw": raw
            }
        })

    def get_jinja(self,model):
        streams = {}
        datasets = {}
        for stream in self.streams_seen:
            tail = make_url('blackbox_tail', 
                stream = stream,
                instance = request.args.get("instance") or ""
            )
            tail_raw = make_url('blackbox_tail_raw', 
                stream = stream,
                instance = request.args.get("instance") or ""
            )
            loglines = model.get_log_lines(stream) or "-"
            streams[stream] = {
                "active": stream in self.streams_enabled,
                "filename": model.get_stream_filename(stream),
                "lines": loglines,
                "tail": tail,
                "tail_raw": tail_raw
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
                "filename": model.get_dataset_filename(stream,dataset),
                "dataset_filename": model.get_rawdata_filename(stream,dataset),
                "summary_url": summary_url,
                "rawdata_url": rawdata_url
            }
        return {
            "streams": streams,
            "datasets": datasets
        }
