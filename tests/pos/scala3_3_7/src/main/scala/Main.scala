import scala.compiletime.erasedValue

inline def typeName[T]: String = inline erasedValue[T] match
  case _: Int    => "Int"
  case _: String => "String"
  case _: Double => "Double"
  case _         => "Unknown"

@main def app(): Unit =
  println(s"Int type: ${typeName[Int]}")
  println(s"String type: ${typeName[String]}")
  println(s"Double type: ${typeName[Double]}")
