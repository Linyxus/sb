// Deeply nested classes and objects
object Outer:
  val outerVal: Int = 1

  object Middle:
    val middleVal: Int = 2

    class Inner(val innerVal: Int):
      def sum: Int = outerVal + middleVal + innerVal

      object DeepInner:
        def deepSum(extra: Int): Int = sum + extra

  class WithCompanion(val x: Int)
  object WithCompanion:
    def apply(x: Int, y: Int): WithCompanion = new WithCompanion(x + y)
    val default: WithCompanion = WithCompanion(0, 0)
