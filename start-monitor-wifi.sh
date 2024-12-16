#!/bin/bash

ifconfig interface_name down
iwconfig interface_name mode monitor
ifconfig interface_name up
iwconfig
