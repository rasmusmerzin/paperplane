module triangle(points) {
  polyhedron(points, [[0, 1, 2]]);
}

module plane(s) {
  translate([0, 0, s / 2]) triangle([[0, s, 0], [s, -s, 0], [-s, -s, 0]]);
  translate([0, 0, -s / 2]) triangle([[0, s, s], [0, -s, 0], [0, -s, s]]);
}

plane(20);

$vpr = [120, 0, 140];
