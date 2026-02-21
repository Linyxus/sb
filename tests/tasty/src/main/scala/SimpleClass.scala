// Basic class with fields and methods
class SimpleClass(val name: String, var age: Int):
  def greet: String = s"Hello, $name"
  def birthday(): Unit = age += 1
  override def toString: String = s"SimpleClass($name, $age)"
