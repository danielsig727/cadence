// Cadence - An extensible Statsd client for Rust!
//
// Copyright 2015 TSH Labs
//
// Available under the MIT license. See LICENSE for details.
//

use std::net::{
    ToSocketAddrs,
    UdpSocket
};

use sinks::{
    MetricSink,
    UdpMetricSink
};

use types::{
    MetricResult,
    Counter,
    Timer,
    Gauge,
    Meter,
    ToMetricString
};


/// Trait for incrementing and decrementing counters.
///
/// Counters are simple values incremented or decremented by a client. The
/// rates at which these events occur or average values will be determined
/// by the server recieving them. Examples of counter uses include number
/// of logins to a system or requests recieved.
///
/// See the [Statsd spec](https://github.com/b/statsd_spec) for more information.
pub trait Counted {
    /// Increment the counter by `1`
    fn incr(&self, key: &str) -> MetricResult<Counter>;

    /// Decrement the counter by `1`
    fn decr(&self, key: &str) -> MetricResult<Counter>;

    /// Increment or decrement the counter by the given amount
    fn count(&self, key: &str, count: i64) -> MetricResult<Counter>;
}


/// Trait for recording timings in milliseconds.
///
/// Timings are a positive number of milliseconds between a start and end
/// time. Examples include time taken to render a web page or time taken
/// for a database call to return.
///
/// See the [Statsd spec](https://github.com/b/statsd_spec) for more information.
pub trait Timed {
    /// Record a timing in milliseconds with the given key
    fn time(&self, key: &str, time: u64) -> MetricResult<Timer>;
}


/// Trait for recording gauge values.
///
/// Gauge values are an instantaneous measurement of a value determined
/// by the client. They do not change unless changed by the client. Examples
/// include things like load average or how many connections are active.
///
/// See the [Statsd spec](https://github.com/b/statsd_spec) for more information.
pub trait Gauged {
    /// Record a gauge value with the given key
    fn gauge(&self, key: &str, value: u64) -> MetricResult<Gauge>;
}


/// Trait for recording meter values.
///
/// Meter values measure the rate at which events occur. These rates are
/// determined by the server, the client simply indicates when they happen.
/// Meters can be thought of as increment-only counters. Examples include
/// things like number of requests handled or number of times something is
/// flushed to disk.
///
/// See the [Statsd spec](https://github.com/b/statsd_spec) for more information.
pub trait Metered {
    /// Record a single metered event with the given key
    fn mark(&self, key: &str) -> MetricResult<Meter>;

    /// Record a meter value with the given key
    fn meter(&self, key: &str, value: u64) -> MetricResult<Meter>;
}


/// Client for Statsd that implements various traits to record metrics.
///
/// The client is the main entry point for users of this library. It supports
/// several traits for recording metrics of different types.
///
/// * `Counted` for emitting counters.
/// * `Timed` for emitting timings.
/// * `Gauged` for emitting gauge values.
/// * `Metered` for emitting meter values.
///
/// For more information about the uses for each type of metric, see the
/// documentation for each mentioned trait.
///
/// The client uses some implementation of a `MetricSink` to emit the metrics.
/// In most cases, users will want to use the `UdpMetricSink` implementation.
pub struct StatsdClient<T: MetricSink> {
    key_gen: KeyGenerator,
    sink: T
}


impl<T: MetricSink> StatsdClient<T> {

    /// Create a new client instance that will use the given prefix for
    /// all metrics emitted to the given `MetricSink` implementation.
    ///
    /// # Example
    ///
    /// ```
    /// use cadence::{StatsdClient, NopMetricSink};
    ///
    /// let prefix = "my.stats";
    /// let client = StatsdClient::from_sink(prefix, NopMetricSink);
    /// ```
    pub fn from_sink(prefix: &str, sink: T) -> StatsdClient<T> {
        StatsdClient{key_gen: KeyGenerator::new(prefix), sink: sink}
    }

    /// Create a new client instance that will use the given prefix to send
    /// metrics to the given host over UDP using an appropriate sink. This is
    /// the contruction method that most users of this library will use.
    ///
    /// **Note** that you must include a type parameter when you call this
    /// method to help the compiler determine the type of `T`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cadence::{StatsdClient, UdpMetricSink};
    ///
    /// let prefix = "my.stats";
    /// let host = ("metrics.example.com", 8125);
    ///
    /// // Note that we include a type parameter for the method call
    /// let client = StatsdClient::<UdpMetricSink>::from_udp_host(prefix, host);
    /// ```
    ///
    /// # Failures
    ///
    /// This method may fail if:
    ///
    /// * It is unable to create a local UDP socket.
    /// * It is unable to resolve the hostname of the metric server.
    /// * The host address is otherwise unable to be parsed.
    pub fn from_udp_host<A>(prefix: &str, host: A) -> MetricResult<StatsdClient<UdpMetricSink>>
        where A: ToSocketAddrs {
        let socket = try!(UdpSocket::bind("0.0.0.0:0"));
        let sink = try!(UdpMetricSink::new(host, socket));
        Ok(StatsdClient::from_sink(prefix, sink))
    }

    // Convert a metric to its Statsd string representation and then send
    // it as UTF-8 bytes to the metric sink. Convert any I/O errors from the
    // sink to MetricResults with the metric itself as a payload for success
    // responses.
    fn send_metric<M: ToMetricString>(&self, metric: M) -> MetricResult<M> {
        let metric_string = metric.to_metric_string();
        let written = try!(self.sink.emit(&metric_string));
        debug!("Wrote {} ({} bytes)", metric_string, written);
        Ok(metric)
    }
}


impl<T: MetricSink> Counted for StatsdClient<T> {
    fn incr(&self, key: &str) -> MetricResult<Counter> {
        self.count(key, 1)
    }

    fn decr(&self, key: &str) -> MetricResult<Counter> {
        self.count(key, -1)
    }

    fn count(&self, key: &str, count: i64) -> MetricResult<Counter> {
        let counter = Counter::new(self.key_gen.make_key(key), count);
        self.send_metric(counter)
    }
}


impl<T: MetricSink> Timed for StatsdClient<T> {
    fn time(&self, key: &str, time: u64) -> MetricResult<Timer> {
        let timer = Timer::new(self.key_gen.make_key(key), time);
        self.send_metric(timer)
    }
}


impl<T: MetricSink> Gauged for StatsdClient<T> {
    fn gauge(&self, key: &str, value: u64) -> MetricResult<Gauge> {
        let gauge = Gauge::new(self.key_gen.make_key(key), value);
        self.send_metric(gauge)
    }
}


impl<T: MetricSink> Metered for StatsdClient<T> {
    fn mark(&self, key: &str) -> MetricResult<Meter> {
        self.meter(key, 1)
    }

    fn meter(&self, key: &str, value: u64) -> MetricResult<Meter> {
        let meter = Meter::new(self.key_gen.make_key(key), value);
        self.send_metric(meter)
    }
}


struct KeyGenerator {
    prefix: String
}


impl KeyGenerator {
    fn new(prefix: &str) -> KeyGenerator {
        let trimmed = if prefix.ends_with('.') {
            prefix.trim_right_matches('.')
        } else {
            prefix
        };

        KeyGenerator{prefix: trimmed.to_string()}
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}.{}", &self.prefix, key)
    }
}


#[cfg(test)]
mod tests {
    use super::{
        KeyGenerator
    };
    
    #[test]
    fn test_key_generator_make_key_with_trailing_dot_prefix() {
        let key_gen = KeyGenerator::new("some.prefix.");
        assert_eq!("some.prefix.a.metric", key_gen.make_key("a.metric"));
    }

    #[test]
    fn test_key_generator_make_key_no_trailing_dot_prefix() {
        let key_gen = KeyGenerator::new("some.prefix");
        assert_eq!("some.prefix.a.metric", key_gen.make_key("a.metric"));
    }
}
