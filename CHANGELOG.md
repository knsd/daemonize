daemonize changelog
===================

Here you can see the full list of changes between each daemonize release.

Version 0.4.1
-------------

Released on March 27, 2019

  * Fix armv7 build, #34

Version 0.4.0
-------------

Released on March 26, 2019

  * Allow an action by the master process right before exit, #33
  * Make privileged action and exit action a FnOnce, #27

Version 0.3.0
-------------

Released on April 07, 2018

  * Don't clobber pidfile of already-running daemon, #21
  * Add ability to `chroot(2)` as part of the daemon process, #22
  * Replace platform-dependent errno with std function, #23
  * Redirect standard streams to defined files, #1

Version 0.2.3
-------------

Released on August 29, 2016

  * Add support for setting a different umask

Version 0.2.2
-------------

Released on March 20, 2016

  * Fixed memory unsafety in CStrings routine
  * Show the actual error on failing to open /dev/null

Version 0.2.1
-------------

Released on January 19, 2016

  * Remove quick-error dependency

Version 0.2.0
-------------

Released on January 17, 2016

  * Add __Nonexhaustive DaemonizeError variant

Version 0.1.2
-------------

Released on January 10, 2016

  * FreeBSD support.

Version 0.1.1
-------------

Released on January 10, 2016

  * Relicense under dual MIT/Apache-2.0.

Version 0.1.0
-------------

Released on December 25, 2015

  * Remove `From<&String>` implementation for `User` and `Group`.
  * Use `umask` from `libc`.

Version 0.0.3
-------------

Released on December 21, 2015

  * Add `Errno` type.
  * Derive some standard traits for types.


Version 0.0.2
-------------

Released on December 21, 2015

  * Add cargo keywords.


Version 0.0.1
-------------

Released on December 21, 2015

  * Initial release.
