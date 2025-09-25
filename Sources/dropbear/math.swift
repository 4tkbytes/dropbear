/// A type alias for making the creation of `Vector3` simple. 
/// 
/// Defaults to `Double`
typealias Vector3D = Math.Vector3<Double>

/** Math library for the dropbear-engine Swift API
* I couldn't seem to find any sort of library that deals with vector math, so why not I create my own
*/
enum Math {
    /// A class containing an `x`, `y` and `z` value of type `T`. 
    /// 
    /// The type `T` must conform as
    /// an `ExpressibleByIntegerLiteral` (no strings or other stuff). 
    class Vector3<T> 
    where T: ExpressibleByIntegerLiteral {
        /// The first value in a `Vector3`
        var x: T
        /// The second value in a `Vector3`
        var y: T
        /// The third value in a `Vector3`
        var z: T

        /// Initialises a new Vector3
        /// - Parameters:
        ///   - x: A value of type T
        ///   - y: A value of type T
        ///   - z: A value of type T
        init(x: T, y: T, z: T) {
            self.x = x
            self.y = y
            self.z = z
        }

        /// Creates a new Vector3 of type T with all values set to zero. 
        /// - Returns: Vector3 of type T
        static func zero() -> Vector3<T> {
            return Vector3(x: 0, y: 0, z: 0)
        }
    }
}