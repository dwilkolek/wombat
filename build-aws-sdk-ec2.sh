if ! test -f /path/to/file; then
    git clone git@github.com:smithy-lang/smithy-rs.git
fi
cd smithy-rs
git pull
sdk use java 17.0.13-amzn
./gradlew :aws:sdk:assemble
cd ..
rm -Rf aws-sdk-ec2
cp -r smithy-rs/aws/sdk/build/aws-sdk/sdk/ec2 aws-sdk-ec2
