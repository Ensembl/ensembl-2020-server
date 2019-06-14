from flask import request, jsonify
import datetime, pytz, tzlocal, yaml, collections, logging

local_timezone = tzlocal.get_localzone()

# need to format it ourselves as python logging doesn't support
# anachronistic log messages
def format_client_time(t):
    t = datetime.datetime.utcfromtimestamp(t/1000.)
    ms = t.microsecond/1000.
    t -= datetime.timedelta(microseconds=t.microsecond)
    t = t.replace(tzinfo=pytz.utc).astimezone(local_timezone)
    return "{0}.{1:03}".format(t.strftime("%Y-%m-%d %H:%M:%S"),int(ms))
        
def safe_filename(fn):
    return "".join([x for x in fn if re.match(r'[\w.-]',x)])

class BAIDebugEndpoint(object):
    def __init__(self,debug_mode_path):
        self.debug_mode_path = debug_mode_path
    
    def post(self):
        with open(self.debug_mode_path) as f:
            debug_config = yaml.load(f)
            streams = []
            datasets = collections.defaultdict(list)
            received = request.get_json()
            inst = received['instance_id']
            
            blackbox_logger = logging.getLogger("blackbox")
            
            # retrieve logs and put into list for sorting, then sort
            for (stream,data) in received['streams'].items():
                for r in data['reports']:
                    streams.append((r['time'],stream,inst,r))
                    if 'dataset' in r:
                        datasets[stream] += r['dataset']
            streams.sort(key=lambda x: x[0])
            
            # write report lines to logger
            loggers = {}
            for (_,stream,inst,r) in streams:
                logger_name = "blackbox.{0}".format(stream)
                if logger_name not in loggers:
                    loggers[logger_name] = logging.getLogger(logger_name)
                logger = loggers[logger_name]
                logger.info(r['text'],extra={
                    "clienttime": format_client_time(r['time']),
                    "streamcode": "{0}/{1}".format(inst,stream),
                    "stack": r['stack']
                })
                            
            # write datasets
            for (filename,data) in datasets.items():
                filename = safe_filename(filename) + ".data"
                print("fn",filename)
                with open(os.path.join(log_path,filename),"ab") as g:
                    g.write("".join(["{} {}\n".format(inst,x) for x in data]).encode("utf-8"))
                    
            return jsonify(debug_config)

def debug_endpoint(bp,debug_mode_path):
    ba_debug = BAIDebugEndpoint(debug_mode_path)
    bp.route("/browser/debug", methods=["POST"])(ba_debug.post)
