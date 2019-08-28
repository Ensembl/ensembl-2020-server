import yaml, os.path

class BAIAPIConfig(object):
    def __init__(self,version,apifile_path):
        self.apifile_path = apifile_path
        self._load(int(version))

    def _load(self,version):
        self.bytecodes = {}
        self._endpoints = {}
        with open(self.apifile_path) as f:
            api = yaml.load(f)
            for subfile in api["bytecode"]:
                with open(os.path.join(os.path.dirname(self.apifile_path),subfile)) as f2:
                    codes = yaml.load(f2)
                    for (k,v) in codes.items():
                        self.bytecodes[k] = v
            self.data_url = api["data-url"]
            if version < 3:
                self._load_endpoints_pre3(api)
            else:
                self._load_endpoints_3plus(api)

    def _load_endpoints_pre3(self, api):
            endpoints = {}
            choices = {}
            ep_cfg_path = api['endpoints']
            ep_cfg_path = os.path.join(os.path.dirname(self.apifile_path),ep_cfg_path)
            with open(ep_cfg_path) as f_ep:
                ep = yaml.load(f_ep)
                for (ep_name,v) in ep['endpoints'].items():
                    if "endpoint" in v and "bytecode" in v:
                        endpoints[ep_name] = (v["endpoint"],v["bytecode"])
                for (track_name,v) in ep['choice'].items():
                    for (species,choices) in v.items():
                        for (code,endpoint) in choices.items():
                            for scale in range(ord(code[0]),ord(code[1])+1):
                                if endpoint in endpoints:
                                    self._endpoints[(species,track_name,chr(scale),"_default")] = endpoints[endpoint]

    def _load_endpoints_3plus(self, api):
            endpoints = {}
            choices = {}
            ep_cfg_path = api['endpoints']
            ep_cfg_path = os.path.join(os.path.dirname(self.apifile_path),ep_cfg_path)
            with open(ep_cfg_path) as f_ep:
                ep = yaml.load(f_ep)
                for (ep_name,v) in ep['endpoints'].items():
                    if "endpoint" in v and "bytecode" in v:
                        endpoints[ep_name] = (v["endpoint"],v["bytecode"])
                for (track_name,vv) in ep['choice'].items():
                    for (focus_type,v) in vv.items():
                        for (species,choices) in v.items():
                            for (code,endpoint) in choices.items():
                                for scale in range(ord(code[0]),ord(code[1])+1):
                                    if endpoint in endpoints:
                                        self._endpoints[(species,track_name,chr(scale),focus_type)] = endpoints[endpoint]

    def get_endpoint(self,chrom,track_name,scale,focus):
        focus_split = focus.split(':') if focus else []
        focus_type = focus_split[1] if len(focus_split) > 1 else "none"
        for focus_check in (focus_type,"_default"):
            for genome_check in (chrom.genome_id,"_default"):
                if (genome_check,track_name,scale,focus_check) in self._endpoints:
                    return self._endpoints[(genome_check,track_name,scale,focus_check)]
        return ("","")

class BAIConfig(object):
    def __init__(self,config_path,assets_path):
        self.config_path = config_path
        self.assets_path = assets_path
        self._endpoints = {}
        self._bytecodes = {}
        self.tracks = {}
        self.focus_specific = {}
        self._load()
        self._api_cache = {}
        
    def _load(self):
        dir_ = os.path.dirname(self.config_path)
        bytecode = {}        
        with open(self.config_path) as f:
            endpoints = {}
            choices = {}
            bc = yaml.load(f)
            self._api = bc["api"]
            ep_cfg_path = bc['endpoints']
            ep_cfg_path = os.path.join(dir_,ep_cfg_path)
            with open(ep_cfg_path) as f_ep:
                ep = yaml.load(f_ep)
                for (ep_name,v) in ep['endpoints'].items():
                    if "endpoint" in v and "bytecode" in v:
                        endpoints[ep_name] = (v["endpoint"],v["bytecode"])
                for (track_name,v) in ep['choice'].items():
                    for (species,choices) in v.items():
                        for (code,endpoint) in choices.items():
                            for scale in range(ord(code[0]),ord(code[1])+1):
                                if endpoint in endpoints:
                                    self._endpoints[(species,track_name,chr(scale))] = endpoints[endpoint]
            for (track_name,v) in bc["tracks"].items():
                if "wire" in v:
                    self.tracks[v["wire"]] = track_name
                self.focus_specific[v["wire"]] = v.get("focus",False)

    def get_api_config(self,version):
        if version not in self._api_cache:
            dir_ = os.path.dirname(self.config_path)
            bytecode_file = os.path.join(dir_,self._api[int(version)])
            self._api_cache[version] = BAIAPIConfig(version,bytecode_file)
        return self._api_cache[version]
