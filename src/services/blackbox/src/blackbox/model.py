import time, json, errno, os.path, urllib, re, datetime, sys

from config import Config

def fmtime(when):
    return datetime.datetime.fromtimestamp(when).strftime("%Y-%m-%d %H:%M:%S")

instance_re = re.compile(r'\[.*?\] \((.*?)\)')

def fscode(text):
    text = urllib.parse.quote(text,safe='')
    text = text.replace('%','@')
    return text

def check_create_dir(blackbox_dir):
    try:
        os.makedirs(blackbox_dir)
    except OSError as e:
        if e.errno == errno.EEXIST:
            pass
        else:
            raise

class Model:
    def __init__(self,blackbox_dir = None):
        if blackbox_dir == None:
            blackbox_dir = os.path.join(
                os.path.abspath(os.environ.get("BLACKBOX_DIR",".")),
                "log"
            )
        check_create_dir(blackbox_dir)
        self.blackbox_dir = blackbox_dir
        self.config = Config(blackbox_dir)

    def stream_path(self,stream):
        stream = fscode(stream)
        return os.path.join(self.blackbox_dir,"stream_{0}.txt".format(stream))

    def dataset_path(self,stream,dataset):
        stream = fscode(stream)
        dataset = fscode(dataset)
        return os.path.join(self.blackbox_dir,"dataset_{0}_{1}.txt".format(stream,dataset))

    def rawdata_path(self,stream,dataset):
        stream = fscode(stream)
        dataset = fscode(dataset)
        return os.path.join(self.blackbox_dir,"rawdata_{0}_{1}.txt".format(stream,dataset))


    def get_stream_filename(self,stream):
        return os.path.split(self.stream_path(stream))[-1]

    def get_dataset_filename(self,stream,dataset):
        return os.path.split(self.dataset_path(stream,dataset))[-1]

    def get_rawdata_filename(self,stream,dataset):
        return os.path.split(self.rawdata_path(stream,dataset))[-1]

    def get_log_lines(self,stream):
        try:
            with open(self.stream_path(stream)) as f:
                return len(f.readlines())
        except OSError as e:
            if e.errno == errno.ENOENT:
                return None
            else:
                raise

    def truncate(self,stream,dataset=None):
        doomed = []
        stream = fscode(stream)
        if dataset:
            prefixes = [ x.format(stream,dataset) for x in ("dataset_{0}_{1}.","rawdata_{0}_{1}.")]
        else:
            prefixes = [ x.format(stream) for x in ("dataset_{0}_","stream_{0}.","rawdata_{0}_")]
        for filename in os.listdir(self.blackbox_dir):
            parts = filename.split("-")
            for prefix in prefixes:
                if filename.startswith(prefix):
                    doomed.append(filename)
        for filename in doomed:
            with open(os.path.join(self.blackbox_dir,filename),"w") as f:
                pass
        return doomed

    def write_line(self,line):
        with open(self.stream_path(line["stream"]),"a") as f:
            text = line["text"]
            if "stack" in line and len(line["stack"]) > 0:
                text = "({0}) {1}".format("/".join(line["stack"]),text)
            f.write("[{0}] ({1}) {2}\n".format(fmtime(line['time']),line['instance'],text))

    def write_dataset(self,line):
        filename = self.dataset_path(line["stream"],line["dataset"])
        columns = ["time","instance","count","total","mean","high","top"]
        data = ""
        if (not os.path.exists(filename)) or os.path.getsize(filename) == 0:
            with open(self.dataset_path(line["stream"],line["dataset"]),"w") as f:
                names = { k: k for k in columns }
                names["high"] = "95%ile"
                data += "\t".join([ names[x] for x in columns ]) + "\n"
        with open(filename,"a") as f:
            data += "\t".join([ str(line[x]) for x in columns ]) + "\n"
            f.write(data)

    def write_data(self,line):
        with open(self.rawdata_path(line["stream"],line["dataset"]),"a") as f:
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

    def get_dataset_data(self,stream,dataset,instance):
        data = ""
        try:
            with open(self.dataset_path(stream,dataset)) as f:
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
        return data

    def get_rawdata_data(self,stream,dataset,instance):
        data = ""
        try:
            with open(self.rawdata_path(stream,dataset)) as f:
                for line in f.readlines():
                    if instance:
                        parts = line.split("\t")
                        if parts[1] != instance:
                            continue
                    data += line
        except OSError as e:
            if e.errno == errno.ENOENT:
                pass
        return data

    def get_log(self,stream,instance):
        path = self.stream_path(stream)        
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
        return out

    def add_mark(self,stream,mark):
        path = self.stream_path(stream)        
        with open(path,"a") as f:
            f.write("[{0}] MARK: {1}\n".format(fmtime(time.time()),mark))
