#!/bin/bash
set -e

if [[ -z "$TRAVIS_JOB_ID" ]]; then
    echo "Cannot run without TRAVIS_JOB_ID in the environment"
    echo "If running locally, please set this to the coveralls job id"
    exit 1
fi

kcov_version=$(kcov --version | cut -f1 -d' ')
if [[ -z $kcov_version ]]; then
    echo "kcov is not installed"
    exit 1
fi

# TODO check kcov version number

# script starts here

project=synth

# print some debugging info
echo "TRAVIS_JOB_ID=$TRAVIS_JOB_ID"
pwd

# clean up any old testers to make sure we only ever upload the latest coverage
rm ./target/debug/$project-*

# then rebuild with the appropriate magical flags set
RUSTFLAGS='-C link-dead-code' cargo test --no-run

echo "attempting to run code coverage utility"
ls target/debug/$project-*

# get a list of files which we actually want to include in the coverage report
# apparently cargo generates a make-style ".d" file which we can possibly use
files=$(cat target/debug/synth.d | cut -f2 -d':' | tail -c +2 | sed -s 's/ /,/g')

for exe in $(ls target/debug/$project-*);
do
    # sometimes files ending in ".d" end up in the output directory, skip them
    if ! [[ -x $exe ]]; then continue; fi

    echo "executing $exe with kcov"
    kcov --include-pattern=$files --verify target/cov --coveralls-id=$TRAVIS_JOB_ID $exe
done
