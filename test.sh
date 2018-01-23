#!/bin/bash

MYSQL_PASS=""
if [[ -z $1 ]]; then
    echo "Error: Please specify your mysql root password or --no-pass"
    echo "usage: $0 (--no-pass|<root-password>)"
    exit 1
else
    if [ "$1" != "--no-pass" ]; then
        MYSQL_PASS="$1"
    fi
fi


set -x

PROJ_DIR=`dirname $0`
mkdir -p "$PROJ_DIR/db"
SQLITE=$PROJ_DIR/db/_test_db.db
POSTGRES=postgres://__migrant_user:pass@localhost/__migrant_testing
MYSQL=mysql://__migrant_user:pass@localhost/__migrant_testing


# Runs a mysql command as root, using a password if provided
# $1 -> command to execute
# $2 -> mysql root password
function do_mysql() {
    if [[ -z $2 ]]; then
        mysql -e "$1"
    else
        mysql -u root --password=$2 -e "$1"
    fi
}

# Setup database tables
# $1 -> mysql root password
function setup() {
    touch "$SQLITE"

    sudo -u postgres createuser __migrant_user
    sudo -u postgres psql -c "alter user __migrant_user with password 'pass'"
    sudo -u postgres createdb __migrant_testing

    do_mysql "create user '__migrant_user'@'localhost' identified by 'pass';" $1
    do_mysql "create database __migrant_testing;" $1
    do_mysql "grant all privileges on __migrant_testing.* to '__migrant_user'@'localhost';" $1
}

# Destroy database tables
# $1 -> mysql root password
function teardown() {
    rm "$SQLITE"

    sudo -u postgres dropdb __migrant_testing
    sudo -u postgres psql -c 'drop user __migrant_user'

    do_mysql "drop user '__migrant_user'@'localhost';" $1
    do_mysql "drop database __migrant_testing;" $1
}

setup $MYSQL_PASS
SQLITE_TEST_CONN_STR=$SQLITE POSTGRES_TEST_CONN_STR=$POSTGRES MYSQL_TEST_CONN_STR=$MYSQL cargo test -- --nocapture
if [ $? -eq 0 ]; then
    teardown $MYSQL_PASS
else
    teardown $MYSQL_PASS
    exit 1
fi

setup $MYSQL_PASS
SQLITE_TEST_CONN_STR=$SQLITE POSTGRES_TEST_CONN_STR=$POSTGRES MYSQL_TEST_CONN_STR=$MYSQL cargo test --features 'sqlite postgresql with-mysql' -- --nocapture
if [ $? -eq 0 ]; then
    teardown $MYSQL_PASS
else
    teardown $MYSQL_PASS
    exit 1
fi

