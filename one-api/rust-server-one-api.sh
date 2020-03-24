#!/bin/sh

SCRIPT="$0"
echo "# START SCRIPT: $SCRIPT"

while [ -h "$SCRIPT" ] ; do
  ls=`ls -ld "$SCRIPT"`
  link=`expr "$ls" : '.*-> \(.*\)$'`
  if expr "$link" : '/.*' > /dev/null; then
    SCRIPT="$link"
  else
    SCRIPT=`dirname "$SCRIPT"`/"$link"
  fi
done

if [ ! -d "${APP_DIR}" ]; then
  APP_DIR=`dirname "$SCRIPT"`/..
  APP_DIR=`cd "${APP_DIR}"; pwd`
fi

executable="./modules/openapi-generator-cli/target/openapi-generator-cli.jar"

if [ ! -f "$executable" ]
then
  mvn -B clean package
fi

#for spec_path in modules/openapi-generator/src/test/resources/*/rust-server/* ; do
for spec_path in  one-api/resources/*.yaml ; do
  export JAVA_OPTS="${JAVA_OPTS} -Xmx1024M -DloggerPath=conf/log4j.properties"
  spec=$(basename "$spec_path" | sed 's/.yaml//')
  if [ "$spec" == "base_types" ]; then
    echo "# SCRIPT: ignore $spec.yaml and continue"
    continue
  fi
  package_name=$spec
  if [ "$spec" == "repos" ]; then
    package_name=repo-api
  fi
  echo "# SCRIPT: generate $spec with package_name=$package_name pwd=$(pwd)"

  args="generate --template-dir modules/openapi-generator/src/main/resources/rust-server
                 --input-spec $spec_path
                 --generator-name rust-server
                 --output one-api/output/rust-server/$spec
                 --additional-properties packageName=$spec
                 --additional-properties hideGenerationTimestamp=true
                 --generate-alias-as-model
                 --package-name $package_name
		 $@"

  java $JAVA_OPTS -jar $executable $args

  if [ $? -ne 0 ]; then
    exit $?
  fi
done
