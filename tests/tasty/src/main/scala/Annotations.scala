// Annotations
class myAnnotation(val msg: String) extends scala.annotation.StaticAnnotation

object Annotations:
  @deprecated("use newMethod instead", "1.0")
  def oldMethod(): Unit = ()

  @myAnnotation("important")
  def annotatedMethod(): Int = 42

  @inline
  def inlinedMethod(x: Int): Int = x * 2

  @volatile
  var sharedVar: Int = 0
