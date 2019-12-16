# Blackbox server

This directory contains an exmaple blackbox server which is implemented as a Python flask app. This app exposes a /data endpoint for blackbox clients and a web forntend for viewing the resulting data. It has no security features as it is designed to be run in a firewalled, dev environment only. Feel free to add auth or https in middleware.

The root url for the browser is `/blackbox/` and for the client is `/blackbox/data`.

To start, run `python src/blackbox`, to run tests, run `python src/blackbox/test.py`.
