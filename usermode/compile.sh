#!/bin/bash

arg=$1
filename=${arg%??}

gcc $1 yehuda-os/helpers.c yehuda-os/sys.c -o ../kernel/bin/$filename -nostdlib
