SHELL := /bin/sh

include ../../config.mk

java-examples: Example.class

%.class:
	javac -cp $(OUT_DIR)/mothra.jar $*.java -d $(OUT_DIR)/

clean:
	rm -rf $(OUT_DIR)/*.class
	rm -rf $(OUT_DIR)/*.jar
