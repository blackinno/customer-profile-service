FROM postgres:16.1

# Generate a couple of common locales. Default is en_US.UTF-8 — override
# POSTGRES_INITDB_ARGS or LANG / LC_ALL on the service if you need a different
# collation (e.g. th_TH.UTF-8 for Thai-language sort order).
RUN apt-get update && apt-get install -y locales && rm -rf /var/lib/apt/lists/* \
    && echo "en_US.UTF-8 UTF-8" > /etc/locale.gen \
    && echo "th_TH.UTF-8 UTF-8" >> /etc/locale.gen \
    && locale-gen

ENV LANG=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8
