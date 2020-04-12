SHELL := /bin/sh

include ../../config.mk

.PHONY : java-examples clean

java-examples: Example.class

%.class:
	javac -cp $(JOUT_DIR)/mothra.jar $*.java -d $(JOUT_DIR)/

clean:
	rm -rf $(JOUT_DIR)