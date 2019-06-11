#! /usr/bin/env python3

from flask import Flask, jsonify
from flask_cors import CORS
import os.path, yaml, logging, logging.config

import browser.browser

app = Flask(__name__)
CORS(app)

data_path = None
yaml_path = None

app_home_dir = os.path.dirname(os.path.realpath(__file__))
yaml_path = os.path.join(app_home_dir, "yaml")
log_path = os.path.join(app_home_dir,"log")

if 'ett_data_path' in os.environ:
    data_path = os.environ['ett_data_path']
else:
    print ("Data path is not set. Some of the endpoint may not work as expected.")

if 'ett_yaml_path' in os.environ:
    yaml_path = os.environ['ett_yaml_path']
else:
    print ("Using default yaml path")

if 'ett_log_path' in os.environ:
    log_path = os.environ['ett_log_path']
else:
    print ("Using default log path")

def configure_logging():
    logging_conf_file = os.path.join(log_path,'logging.conf')
    if os.path.exists(logging_conf_file):
        # temporary chdir ensures config-file paths relative to log_path
        old_cwd = os.getcwd()
        os.chdir(log_path)
        logging.config.fileConfig(logging_conf_file)
        os.chdir(old_cwd)
        general_logger = logging.getLogger("general")
        general_logger.info("logging configured from {0}".format(logging_conf_file))
    else:
        default_logfile = os.path.join(log_path,"server.log")
        logging.basicConfig(filename=default_logfile,level=logging.DEBUG)
        general_logger = logging.getLogger("general")
        general_logger.info("no logging config found. Using basic config")

configure_logging()
general_logger = logging.getLogger("general")

assets_path = os.path.join(app_home_dir,"assets")
config_path = os.path.join(yaml_path,"config.yaml")
objects_list_path = os.path.join(yaml_path,"example_objects.yaml")
objects_info_path = os.path.join(yaml_path,"objects_info.yaml")

browser_image_bp = browser.browser.browser_setup(yaml_path,data_path,assets_path)
app.register_blueprint(browser_image_bp)

@app.route("/browser/example_objects")
def example_objects():
    with open(objects_list_path) as f:
        data = yaml.load(f)
        return jsonify(data)


@app.route("/browser/get_object_info/<object_id>")
def get_object_info(object_id):
    with open(objects_info_path) as f:
        data = yaml.load(f)
        if object_id not in data:
          return jsonify({'error':'Object Not Found'})
        else:
          return jsonify(data[object_id])
    
if __name__ == "__main__":
   app.run(port=4000)
