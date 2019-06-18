FROM python:3.5

# maintainer of the image
LABEL maintainer="kamal@ebi.ac.uk"

# Environment variable
ENV PYTHONUNBUFFERED TRUE

RUN mkdir -p /usr/src/app

WORKDIR /usr/src/app

COPY src/services/general/src/ /usr/src/app/
COPY requirements.txt /usr/src/app/

RUN pip3 install --no-cache-dir -r requirements.txt

EXPOSE 4000

CMD ["gunicorn","--bind=0.0.0.0:4000","server:app"]