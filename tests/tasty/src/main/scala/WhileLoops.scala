// While loops and mutable state
object WhileLoops:
  def gcd(a: Int, b: Int): Int =
    var x = a.abs
    var y = b.abs
    while y != 0 do
      val t = y
      y = x % y
      x = t
    x

  def collatzLength(start: Int): Int =
    var n = start
    var steps = 0
    while n != 1 do
      if n % 2 == 0 then n = n / 2
      else n = 3 * n + 1
      steps += 1
    steps

  def fibonacci(n: Int): Long =
    if n <= 1 then n.toLong
    else
      var a = 0L
      var b = 1L
      var i = 2
      while i <= n do
        val c = a + b
        a = b
        b = c
        i += 1
      b
