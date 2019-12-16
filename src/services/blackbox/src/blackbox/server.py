#! /usr/bin/env python3

from dotenv import load_dotenv
from flask import Flask, jsonify, request, send_from_directory, render_template, redirect, Response, Blueprint, url_for
import sys, os, os.path, yaml, logging, logging.config, errno, urllib, datetime, time, collections,json, re
from base64 import b64encode

from config import Config
from model import Model
from util import page_path

script_dir = os.path.dirname(os.path.realpath(__file__))

"""
TODO

local assets
OpenAPI
tests
README

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

def xxx_make_fake_config():
    config = Config(blackbox_dir)
    config.enable("test1")
    config.disable("test2")
    config.enable_raw("test1","raw")
    config.disable_raw("test1","noraw")
    return config

def blackbox():
    model = Model(blackbox_dir)
    bb = Blueprint('blackbox', __name__,template_folder='templates')


    @bb.route('/')
    def blackbox_control():
        instance = request.args.get("instance") or ""
        return render_template("index.html", config = model.config.get_jinja(model),instance = instance)

    @bb.route("/dataset")
    def blackbox_dataset():
        stream = request.args.get("stream")
        dataset = request.args.get("dataset")
        instance = request.args.get("instance")
        data = model.get_dataset_data(stream,dataset,instance)
        return Response(data,mimetype="text/plain")

    @bb.route("/rawdata")
    def blackbox_rawdata():
        stream = request.args.get("stream")
        dataset = request.args.get("dataset")
        instance = request.args.get("instance")
        data = model.get_rawdata_data(stream,dataset,instance)
        return Response(data,mimetype="text/plain")

    @bb.route("/tail")
    def blackbox_tail():
        stream = request.args.get("stream")
        instance = request.args.get("instance") or ""
        out = model.get_log(stream,instance)
        return render_template("tail.html", data = out)

    @bb.route("/mark",methods=["POST"])
    def blackbox_mark():
        stream = request.form["stream"]
        mark = request.form["mark"]
        model.add_mark(stream,mark)
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
            elif key == "delete":
                stream = request.form["delete"]
                model.config.delete_stream(stream)
                model.truncate(stream)
            elif key == "dataset-delete":
                stream = request.form["stream"]
                dataset = request.form["dataset-delete"]
                model.config.delete_dataset(stream,dataset)
                model.truncate(stream,dataset)

        model.config.save()
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

    @bb.route("/data", methods=["POST","GET"])
    def blackbox_data():
        if request.method == "POST":
            incoming = request.get_json(force=True)
            for line in incoming:
                model.process_line(line)
        return model.config.to_json()

    @bb.route("/assets/<path:asset>")
    def blackbox_asset(asset):
        print("ASSETS",asset)
        return send_from_directory(os.path.join(script_dir,"../../assets"),asset)

    return bb
