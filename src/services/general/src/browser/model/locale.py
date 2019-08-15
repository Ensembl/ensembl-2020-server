# A locale is where you should go to see an object in all its glory.
# It's a browser thing not a bilogical thing because things like
# padding etc are designed elements not biological properties, hence
# apparent (but not actual) duplication. There's no reason why longer-
# term it couldn't be wired to a REST service, etc, though, with
# wrappers and caches here. But it should always be its own endpoint.
# For now it just comes from the files, though.

import os.path, dbm, tempfile

class Locale(object):
    def __init__(self):
        self._backing_file = os.path.join(
            tempfile.gettempdir(),
            "locale.dbm")
        self._data = dbm.open(self._backing_file,"c")
        
    def add_locale(self,id_,stick,start,end):
        self._data[id_.encode("utf8")] = ":".join([stick,str(start),str(end)])

    def get_locale(self,id_):
        parts = id_.split(':',2)
        if parts[1] == 'region':
            region = parts[2].split(':')
            (start,end) = region[1].split('-')
            return (parts[0]+":"+region[0],int(start),int(end))
        else:
            out = self._data.get(id_.encode("utf8")).decode("utf8").rsplit(':',2)
            return [out[0],int(out[1]),int(out[2])]
