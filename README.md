# dst updater

As dormakaba one of the topic is to update time when the dst comes
The idea of the PoC is to have a lambda fetching all the devices needing a dst update and sending the dst to all of them

The goal of the PoC is to compare lambda rust and lambda node
The code here is not representative of a real case scenario

## architecture

One HTTP lambda woked up through a GET call running a set of dst zones saved in the code

LATER - The lambda is getting the info, perform a batch get to dynDB and send

## running the program

you need to install cargo lambda on top of cargo:
<https://github.com/cargo-lambda/cargo-lambda>

To compile for lambda run:

`cargo lambda build --release`

then zip the file named bootstrap and upload it to your lambda. 
__Important__: the zip need to be named bootstrap to work

for more information regarding rust lambda go here:

<https://aws.amazon.com/blogs/opensource/rust-runtime-for-aws-lambda/>

## TO DO

[X] - multi threading in lambda ?
    Multi threading is supported by lambda and they encourage to use it for low memory low cpu languages

[-] - what happens with the worker of a lambda failing ?
    Still not clear if we should change the clean up or not of a lambda

[-] - need to work on batch insert for better perf

[-] - need to finalize dyndb insert
