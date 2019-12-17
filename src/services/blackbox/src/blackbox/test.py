import os, unittest, tempfile, shutil, json

from html.parser import HTMLParser
from flask import Flask
from server import blackbox

SMOKE_DATA = json.dumps({
    "streams": {
        "test": ["raw","raw2"],
        "test2": ["raw3","raw4"],
    },
    "records": [
        {
            "instance":"test1",
            "stack":["a","b"],
            "text":"Hello, world!",
            "time":2.0, 
            "stream": "test"
        },
        {
            "stream": "test",
            "data":[2.0],
            "ago": [0],
            "dataset":"raw",
            "instance":"test1",
            "text":"raw elapsed: num=1 total=2.00units avg=2.00units 95%ile=2.00units top=2.00units",
            "time":2.0,
            "count": 1,
            "total": 2,
            "mean": 2,
            "high": 2,
            "top": 2
        },
            {
            "stream": "test",
            "data":[2.0],
            "ago": [0],
            "dataset":"raw",
            "instance":"test2",
            "text":"raw elapsed: num=2 total=1.00units avg=1.00units 95%ile=1.00units top=1.00units",
            "time": 3.0,
            "count": 2,
            "total": 1,
            "mean": 1,
            "high":1,
            "top": 1
        }
    ]
})

def convert_line(line):
    return [int(float(x)) if i != 1 else x for (i,x) in enumerate(line.split("\t"))]

class TagExtractor(HTMLParser):
    def __init__(self,tag):
        super().__init__()
        self.tag = tag
        self.active = False
        self.current = ""
        self.output = []
    
    def handle_starttag(self, tag, attrs):
        if tag == self.tag:
            self.active = True

    def handle_endtag(self, tag):
        if tag == self.tag:
            self.active = False
            self.output.append(self.current)
            self.current = ""

    def handle_data(self, data):
        if self.active:
            self.current += data

def tag_extract(tag,text):
    tx = TagExtractor(tag)
    tx.feed(text.decode("utf8"))
    return tx.output

class BasicTests(unittest.TestCase): 
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self.app = Flask(__name__)
        self.app.register_blueprint(blackbox(self.tmpdir),url_prefix="/blackbox")
        self.client = self.app.test_client()

    def tearDown(self):
        shutil.rmtree(self.tmpdir)

    def _get_tail(self,stream,instance=''):
        response = self.client.get('/blackbox/tail',query_string={
            "stream": stream,
            "instance": instance
        })
        self.assertEqual(response.status_code,200)
        return tag_extract("pre",response.data)[0].strip().split("\n")

    def _get_dataset(self,stream,dataset,instance=''):
        response = self.client.get('/blackbox/dataset',query_string={
            "stream": stream,
            "dataset": dataset,
            "instance": instance
        })
        self.assertEqual(response.status_code,200)
        return response.data.decode("utf8").strip().split("\n")

    def _get_rawdata(self,stream,dataset,instance=''):
        response = self.client.get('/blackbox/rawdata',query_string={
            "stream": stream,
            "dataset": dataset,
            "instance": instance
        })
        self.assertEqual(response.status_code,200)
        return response.data.decode("utf8").strip().split("\n")


    def test_config(self):
        response = self.client.post('/blackbox/update-config',data = {
            "enable": "test1"
        })
        self.assertEqual(response.status_code,200)

    def test_smoke(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        self.assertEqual(response.status_code,200)
        # check logs
        text = self._get_tail("test")
        self.assertEqual([
            '[1970-01-01 01:00:00.002] (test1) (a/b) Hello, world!', 
            '[1970-01-01 01:00:00.002] (test1) raw elapsed: num=1 total=2.00units avg=2.00units 95%ile=2.00units top=2.00units',
            '[1970-01-01 01:00:00.003] (test2) raw elapsed: num=2 total=1.00units avg=1.00units 95%ile=1.00units top=1.00units'
        ],text)
        # check summaries
        dataset = self._get_dataset("test","raw")
        self.assertEqual(len(dataset),3)
        self.assertEqual("\t".join(["time","instance","count","total","mean","95%ile","top"]),dataset[0])
        self.assertEqual([2,"test1",1,2,2,2,2],convert_line(dataset[1]))
        self.assertEqual([3,"test2",2,1,1,1,1],convert_line(dataset[2]))
        # check raw data
        rawdata = self._get_rawdata("test","raw")
        self.assertEqual(len(rawdata),2)
        self.assertEqual([2,'test1',2],convert_line(rawdata[0]))
        self.assertEqual([3,'test2',2],convert_line(rawdata[1]))
 
    def test_page(self):
        page = self.client.get('/blackbox/')
        self.assertRegex(page.data.decode("utf8"),r'onsubmit="')

    def test_mark(self):
        response = self.client.post('/blackbox/mark',data={
            'stream': 'test',
            'mark': 'test mark'
        })
        text = self._get_tail("test")
        self.assertEqual(len(text),1)
        self.assertRegex(text[0],'MARK: test mark')

    def test_update_config(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        response = self.client.post('/blackbox/update-config',data={
            'stream': 'test',
            'disable': 'me'
        })
        cfg2 = json.loads(response.data.decode("utf8"))
        cfg = json.loads(self.client.get('/blackbox/data').data.decode("utf8"))
        self.assertEqual(cfg,cfg2)
        self.assertFalse("me" in cfg['config']['enable'])
        response = self.client.post('/blackbox/update-config',data={
            'stream': 'test',
            'enable': 'me'
        })
        cfg = json.loads(self.client.get('/blackbox/data').data.decode("utf8"))
        self.assertTrue("me" in cfg['config']['enable'])
        response = self.client.post('/blackbox/update-config',data={
            'stream': 'test',
            'raw-enable': 'mee'
        })
        cfg = json.loads(self.client.get('/blackbox/data').data.decode("utf8"))
        self.assertTrue("mee" in cfg['config']['raw']['test'])
        response = self.client.post('/blackbox/update-config',data={
            'stream': 'test',
            'raw-disable': 'mee'
        })
        cfg = json.loads(self.client.get('/blackbox/data').data.decode("utf8"))
        self.assertFalse("mee" in cfg['config']['raw']['test'])
        self.assertEqual(3,len(self._get_dataset("test","raw")))
        self.client.post('/blackbox/update-config',data={
            'stream': 'test',
            'dataset-delete': 'raw'
        })
        self.assertEqual(1,len(self._get_dataset("test","raw")))
        self.assertEqual(3,len(self._get_tail("test")))
        self.client.post('/blackbox/update-config',data={
            'delete': 'test',
        })
        self.assertEqual(1,len(self._get_tail("test")))

    def test_update_config_page(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        page = self.client.post('/blackbox/update-config-page',data={
            'stream': 'test',
            'disable': 'me'
        },follow_redirects=True)
        self.assertRegex(page.data.decode("utf8"),r'onsubmit="')

    def test_instance_log(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        text = self._get_tail("test")
        self.assertEqual([
            '[1970-01-01 01:00:00.002] (test1) (a/b) Hello, world!', 
            '[1970-01-01 01:00:00.002] (test1) raw elapsed: num=1 total=2.00units avg=2.00units 95%ile=2.00units top=2.00units',
            '[1970-01-01 01:00:00.003] (test2) raw elapsed: num=2 total=1.00units avg=1.00units 95%ile=1.00units top=1.00units'
        ],text)
        text = self._get_tail("test","test1")
        self.assertEqual([
            '[1970-01-01 01:00:00.002] (test1) (a/b) Hello, world!', 
            '[1970-01-01 01:00:00.002] (test1) raw elapsed: num=1 total=2.00units avg=2.00units 95%ile=2.00units top=2.00units'
        ],text)
        text = self._get_tail("test","test2")
        self.assertEqual([
            '[1970-01-01 01:00:00.003] (test2) raw elapsed: num=2 total=1.00units avg=1.00units 95%ile=1.00units top=1.00units'
        ],text)

    def test_instance_dataset(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        dataset = self._get_dataset("test","raw")
        self.assertEqual(len(dataset),3)
        self.assertEqual(int(dataset[1].split("\t")[2]),1)
        self.assertEqual(int(dataset[2].split("\t")[2]),2)
        dataset = self._get_dataset("test","raw","test1")
        self.assertEqual(len(dataset),2)
        self.assertEqual(int(dataset[1].split("\t")[2]),1)
        dataset = self._get_dataset("test","raw","test2")
        self.assertEqual(len(dataset),2)
        self.assertEqual(int(dataset[1].split("\t")[2]),2)

    def test_instance_raw(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        rawdata = self._get_rawdata("test","raw")
        self.assertEqual(len(rawdata),2)
        self.assertEqual([2,'test1',2],convert_line(rawdata[0]))
        self.assertEqual([3,'test2',2],convert_line(rawdata[1]))
        rawdata = self._get_rawdata("test","raw","test1")
        self.assertEqual(len(rawdata),1)
        self.assertEqual([2,'test1',2],convert_line(rawdata[0]))
        rawdata = self._get_rawdata("test","raw","test2")
        self.assertEqual(len(rawdata),1)
        self.assertEqual([3,'test2',2],convert_line(rawdata[0]))

    def test_truncate(self):
        response = self.client.post('/blackbox/data',data=SMOKE_DATA)
        text = self._get_tail("test")
        self.assertNotEqual('',text[0])
        self.client.post('/blackbox/update-config',data={
            'truncate': 'test',
        })
        text = self._get_tail("test")
        self.assertEqual('',text[0])

    def test_assets(self):
        response = self.client.get('/blackbox/assets/plotly-latest.min.js')
        self.assertEqual(response.status_code,200)
        self.assertRegex(response.data.decode("utf8"),'Open Sans')
        response.close()

    def test_prepopulation(self):
        self.client.post('/blackbox/data',data=SMOKE_DATA)
        self.client.post('/blackbox/update-config',data={
            'enable': 'test2'
        })
        response = self.client.get('/blackbox/')
        cells = tag_extract("td",response.data)
        self.assertTrue("raw3" in cells)

if __name__ == "__main__":
    unittest.main()
