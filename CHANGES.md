# Changes

## [2.0.0] - 2024-05-28

* Use ntex-service 3.0

## [1.0.0] - 2024-01-09

* Fix decode array #8

## [1.0.0-b.0] - 2024-01-07

* Use "async fn" in trait for Service definition

## [0.5.1] - 2023-06-23

* Fix client connector usage, fixes lifetime constraint

## [0.5.0] - 2023-06-22

* Migrate to ntex 0.7

## [0.4.1] - 2023-01-28

* Fix decode uncomple array data

## [0.4.0] - 2023-01-04

* 0.4 Release

## [0.4.0-beta.0] - 2022-12-28

* Migrate to ntex-service-1.0

## [0.3.3] - 2022-07-07

* pubsub support #5

## [0.3.2] - 2022-05-04

* Add `Keys` command #4

* Add Clone trait impl to CommandError

## [0.3.1] - 2022-01-10

* Simplify connector type, drop .boxed_connector() method

* Remove openssl and rustls features

## [0.3.0] - 2021-12-30

* Upgrade to ntex 0.5.0

## [0.3.0-b.3] - 2021-12-25

* Add RedisConnector::boxed_connector()

## [0.3.0-b.2] - 2021-12-24

* Update service trait

## [0.3.0-b.1] - 2021-12-22

* Upgrade to ntex 0.5.0 b.2

## [0.3.0-b.0] - 2021-12-19

* Upgrade to ntex 0.5

## [0.2.4] - 2021-12-02

* Add memory pools support

## [0.2.3] - 2021-09-03

* Fix panic during protocol decoding

## [0.2.2] - 2021-08-28

* use new ntex's timer api

## [0.2.1] - 2021-08-20

* cmd: Add Select and Ping commands

## [0.2.0] - 2021-06-27

* upgrade to ntex-0.4

## [0.1.3] - 2021-05-05

* cmd: Rename HSet::insert() to HSet::entry()

## [0.1.2] - 2021-04-21

* use ntex::framed for redis transports

## [0.1.1] - 2021-04-04

* upgrade ntex, drop direct futures dependency

## [0.1.0] - 2021-03-10

* Initial release
