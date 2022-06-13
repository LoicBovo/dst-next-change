# dst updater

As dormakaba one of the topic is to update time when the dst comes
The idea of the PoC is to have a lambda fetching all the devices needing a dst update and sending the dst to all of them

The goal of the PoC is to compare lambda rust and lambda node
The code here is not representative of a real case scenario

## architecture

One HTTP lambda woked up through a GET call providing the dst zone to send
The lambda is getting the info, perform a batch get to dynDB and send

## dst logic

for each day of the year for each timezone, we check if there is a dst change, if there is one, we do save the day into db if not we iterate until finding one, if none found return empty

spawn tasks to do the dst finding thing then write into a stream, the stream one take care of saving into the db

## to do

[] - multi threading in lambda ?
[] - what happens with the worker of a lambda failing ?
[] - make it spawn parrallel tasks to go faster: <https://stackoverflow.com/questions/63434977/how-can-i-spawn-asynchronous-methods-in-a-loop>
