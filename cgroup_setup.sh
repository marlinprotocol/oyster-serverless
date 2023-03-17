#!/bin/sh
cgcreate -g memory:workerd1
sudo cgset -r memory.max=100M workerd1

cgcreate -g memory:workerd2
sudo cgset -r memory.max=100M workerd2

cgcreate -g memory:workerd3
sudo cgset -r memory.max=100M workerd3

cgcreate -g memory:workerd4
sudo cgset -r memory.max=100M workerd4

cgcreate -g memory:workerd5
sudo cgset -r memory.max=100M workerd5