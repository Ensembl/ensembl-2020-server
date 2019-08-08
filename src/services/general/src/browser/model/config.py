import yaml, os.path

class BAIAPIConfig(object):
    def __init__(self,apifile_path):
        self.apifile_path = apifile_path
        self._load()

    def _load(self):
        self.bytecodes = {}
        with open(self.apifile_path) as f:
            api = yaml.load(f)
            for subfile in api["bytecode"]:
                with open(os.path.join(os.path.dirname(self.apifile_path),subfile)) as f2:
                    codes = yaml.load(f2)
                    for (k,v) in codes.items():
                        self.bytecodes[k] = v
            self.data_url = api["data-url"]

class BAIConfig(object):
    def __init__(self,config_path,assets_path):
        self.config_path = config_path
        self.assets_path = assets_path
        self._endpoints = {}
        self._bytecodes = {}
        self.tracks = {}
        self._load()
        
    def _load(self):
        dir_ = os.path.dirname(self.config_path)
        bytecode = {}        
        with open(self.config_path) as f:
            ep_map = {}
            bc_map = {}
            choices = {}
            bc = yaml.load(f)
            self._api = bc["api"]
            ep_cfg_path = bc['endpoints']
            ep_cfg_path = os.path.join(dir_,ep_cfg_path)
            with open(ep_cfg_path) as f_ep:
                ep = yaml.load(f_ep)
                for (ep_name,v) in ep['endpoints'].items():
                    if "endpoint" in v:
                        ep_map[ep_name] = v["endpoint"]
                    if "bytecode" in v:
                        bc_map[ep_name] = v["bytecode"]                        
                for (track_name,v) in ep['choice'].items():
                    for (species,choices) in v.items():
                        for (code,endpoint) in choices.items():
                            for scale in range(ord(code[0]),ord(code[1])+1):
                                if endpoint in ep_map and endpoint in bc_map:
                                    self._endpoints[(species,track_name,chr(scale))] = (ep_map[endpoint],bc_map[endpoint])
            for (track_name,v) in bc["tracks"].items():
                if "wire" in v:
                    self.tracks[v["wire"]] = track_name

    def get_endpoint(self,chrom,track_name,scale):
        for check in (chrom.genome_id,"_default"):
            if (check,track_name,scale) in self._endpoints:
                return self._endpoints[(check,track_name,scale)]
        return ("","")

    def get_api_config(self,version):
        dir_ = os.path.dirname(self.config_path)
        bytecode_file = os.path.join(dir_,self._api[int(version)])
        return BAIAPIConfig(bytecode_file)

