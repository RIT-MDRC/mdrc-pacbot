Object.assign(window.search, {"doc_urls":["the_grid.html#the-grid","the_grid.html#the-logical-grid","the_grid.html#coordinates"],"index":{"documentStore":{"docInfo":{"0":{"body":2,"breadcrumbs":2,"title":1},"1":{"body":22,"breadcrumbs":3,"title":2},"2":{"body":107,"breadcrumbs":2,"title":1}},"docs":{"0":{"body":"Rust API","breadcrumbs":"The Grid » The Grid","id":"0","title":"The Grid"},"1":{"body":"The logical grid represents the pathways where Pacman and the ghost can travel in integer coordinates. The actual Pacbot grid is 28 cells wide by 31 cells tall, but this code supports 32x32 grids.","breadcrumbs":"The Grid » The Logical Grid","id":"1","title":"The Logical Grid"},"2":{"body":"+x = right +y = up Points are always given as (x, y). Both x and y are represented as u8 to save space. The 2-dimensional list that represents the grid should be indexed like grid[x][y]. To avoid confusion, instead of indexing directly, it is recommended to only use ComputedGrid.at() with a Point2. Note that the array looks sideways when viewed in an editor: [ [bottom left, ..., top left], ..., [bottom right, ..., top right]\n] When displayed in a running program, (0, 0) should always be in the bottom left. This matches the official visualization from Harvard Robotics. So this grid: [ [0, 1, 2], [3, 4, 5], [6, 7, 8],\n] Would appear like this: 2 5 8\n1 4 7\n0 3 6 On the official Pacbot grid: (1, 1) is the bottom left corner (26, 1) is the bottom right corner (1, 29) is the top left corner (26, 29) is the top right corner Pacbot's starting position is (14, 7).","breadcrumbs":"The Grid » Coordinates","id":"2","title":"Coordinates"}},"length":3,"save":true},"fields":["title","body","breadcrumbs"],"index":{"body":{"root":{"0":{"df":1,"docs":{"2":{"tf":2.0}}},"1":{"4":{"df":1,"docs":{"2":{"tf":1.0}}},"df":1,"docs":{"2":{"tf":2.449489742783178}}},"2":{"6":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"8":{"df":1,"docs":{"1":{"tf":1.0}}},"9":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"df":1,"docs":{"2":{"tf":1.7320508075688772}}},"3":{"1":{"df":1,"docs":{"1":{"tf":1.0}}},"2":{"df":0,"docs":{},"x":{"3":{"2":{"df":1,"docs":{"1":{"tf":1.0}}},"df":0,"docs":{}},"df":0,"docs":{}}},"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"4":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"5":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"6":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"7":{"df":1,"docs":{"2":{"tf":1.7320508075688772}}},"8":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"a":{"c":{"df":0,"docs":{},"t":{"df":0,"docs":{},"u":{"a":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}}},"df":0,"docs":{},"l":{"df":0,"docs":{},"w":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}}},"df":0,"docs":{}}},"p":{"df":0,"docs":{},"i":{"df":1,"docs":{"0":{"tf":1.0}}},"p":{"df":0,"docs":{},"e":{"a":{"df":0,"docs":{},"r":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}},"r":{"df":0,"docs":{},"r":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}},"v":{"df":0,"docs":{},"o":{"df":0,"docs":{},"i":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}}}}},"b":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"df":0,"docs":{},"h":{"df":1,"docs":{"2":{"tf":1.0}}},"t":{"df":0,"docs":{},"o":{"df":0,"docs":{},"m":{"df":1,"docs":{"2":{"tf":2.23606797749979}}}}}}}},"c":{"df":0,"docs":{},"e":{"df":0,"docs":{},"l":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.4142135623730951}}}}},"o":{"d":{"df":0,"docs":{},"e":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{},"m":{"df":0,"docs":{},"p":{"df":0,"docs":{},"u":{"df":0,"docs":{},"t":{"df":0,"docs":{},"e":{"d":{"df":0,"docs":{},"g":{"df":0,"docs":{},"r":{"df":0,"docs":{},"i":{"d":{".":{"a":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}},"df":0,"docs":{}},"df":0,"docs":{}}}}},"df":0,"docs":{}}}}}},"n":{"df":0,"docs":{},"f":{"df":0,"docs":{},"u":{"df":0,"docs":{},"s":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"o":{"df":0,"docs":{},"r":{"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"n":{"df":2,"docs":{"1":{"tf":1.0},"2":{"tf":1.0}}}}},"df":0,"docs":{}}},"r":{"df":0,"docs":{},"n":{"df":0,"docs":{},"e":{"df":0,"docs":{},"r":{"df":1,"docs":{"2":{"tf":2.0}}}}}}}},"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"m":{"df":0,"docs":{},"e":{"df":0,"docs":{},"n":{"df":0,"docs":{},"s":{"df":0,"docs":{},"i":{"df":0,"docs":{},"o":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}}}}}}}},"r":{"df":0,"docs":{},"e":{"c":{"df":0,"docs":{},"t":{"df":0,"docs":{},"l":{"df":0,"docs":{},"i":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"df":0,"docs":{}}},"s":{"df":0,"docs":{},"p":{"df":0,"docs":{},"l":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}}}},"df":0,"docs":{},"e":{"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"t":{"df":0,"docs":{},"o":{"df":0,"docs":{},"r":{"df":1,"docs":{"2":{"tf":1.0}}}}}}},"df":0,"docs":{}},"g":{"df":0,"docs":{},"h":{"df":0,"docs":{},"o":{"df":0,"docs":{},"s":{"df":0,"docs":{},"t":{"df":1,"docs":{"1":{"tf":1.0}}}}}},"i":{"df":0,"docs":{},"v":{"df":0,"docs":{},"e":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"r":{"df":0,"docs":{},"i":{"d":{"[":{"df":0,"docs":{},"x":{"]":{"[":{"df":0,"docs":{},"i":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}},"df":0,"docs":{}}},"df":3,"docs":{"0":{"tf":1.0},"1":{"tf":2.0},"2":{"tf":1.7320508075688772}}},"df":0,"docs":{}}}},"h":{"a":{"df":0,"docs":{},"r":{"df":0,"docs":{},"v":{"a":{"df":0,"docs":{},"r":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}}},"df":0,"docs":{}}}},"df":0,"docs":{}},"i":{"df":0,"docs":{},"n":{"d":{"df":0,"docs":{},"e":{"df":0,"docs":{},"x":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}}}},"df":0,"docs":{},"s":{"df":0,"docs":{},"t":{"df":0,"docs":{},"e":{"a":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}},"df":0,"docs":{}}}},"t":{"df":0,"docs":{},"e":{"df":0,"docs":{},"g":{"df":1,"docs":{"1":{"tf":1.0}}}}}}},"l":{"df":0,"docs":{},"e":{"df":0,"docs":{},"f":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":2.23606797749979}}}}},"i":{"df":0,"docs":{},"s":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}},"o":{"df":0,"docs":{},"g":{"df":0,"docs":{},"i":{"c":{"df":1,"docs":{"1":{"tf":1.4142135623730951}}},"df":0,"docs":{}}},"o":{"df":0,"docs":{},"k":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"m":{"a":{"df":0,"docs":{},"t":{"c":{"df":0,"docs":{},"h":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}},"df":0,"docs":{}},"n":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"df":0,"docs":{},"e":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"o":{"df":0,"docs":{},"f":{"df":0,"docs":{},"f":{"df":0,"docs":{},"i":{"c":{"df":0,"docs":{},"i":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}}},"df":0,"docs":{}}}}},"p":{"a":{"c":{"b":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"'":{"df":1,"docs":{"2":{"tf":1.0}}},"df":2,"docs":{"1":{"tf":1.0},"2":{"tf":1.0}}}}},"df":0,"docs":{},"m":{"a":{"df":0,"docs":{},"n":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}},"df":0,"docs":{},"t":{"df":0,"docs":{},"h":{"df":0,"docs":{},"w":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}}}},"df":0,"docs":{},"o":{"df":0,"docs":{},"i":{"df":0,"docs":{},"n":{"df":0,"docs":{},"t":{"2":{"df":1,"docs":{"2":{"tf":1.0}}},"df":1,"docs":{"2":{"tf":1.0}}}}},"s":{"df":0,"docs":{},"i":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"r":{"df":0,"docs":{},"o":{"df":0,"docs":{},"g":{"df":0,"docs":{},"r":{"a":{"df":0,"docs":{},"m":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}}}},"r":{"df":0,"docs":{},"e":{"c":{"df":0,"docs":{},"o":{"df":0,"docs":{},"m":{"df":0,"docs":{},"m":{"df":0,"docs":{},"e":{"df":0,"docs":{},"n":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}}}}}}},"df":0,"docs":{},"p":{"df":0,"docs":{},"r":{"df":0,"docs":{},"e":{"df":0,"docs":{},"s":{"df":2,"docs":{"1":{"tf":1.0},"2":{"tf":1.4142135623730951}}}}}}},"i":{"df":0,"docs":{},"g":{"df":0,"docs":{},"h":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":2.23606797749979}}}}}},"o":{"b":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{}},"u":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}},"s":{"df":0,"docs":{},"t":{"df":1,"docs":{"0":{"tf":1.0}}}}}},"s":{"a":{"df":0,"docs":{},"v":{"df":0,"docs":{},"e":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{},"i":{"d":{"df":0,"docs":{},"e":{"df":0,"docs":{},"w":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}},"df":0,"docs":{}},"p":{"a":{"c":{"df":0,"docs":{},"e":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}},"df":0,"docs":{}},"t":{"a":{"df":0,"docs":{},"r":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{}},"u":{"df":0,"docs":{},"p":{"df":0,"docs":{},"p":{"df":0,"docs":{},"o":{"df":0,"docs":{},"r":{"df":0,"docs":{},"t":{"df":1,"docs":{"1":{"tf":1.0}}}}}}}}},"t":{"a":{"df":0,"docs":{},"l":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.0}}}}},"df":0,"docs":{},"o":{"df":0,"docs":{},"p":{"df":1,"docs":{"2":{"tf":2.0}}}},"r":{"a":{"df":0,"docs":{},"v":{"df":0,"docs":{},"e":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.0}}}}}},"df":0,"docs":{}}},"u":{"8":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{},"p":{"df":1,"docs":{"2":{"tf":1.0}}},"s":{"df":1,"docs":{"2":{"tf":1.0}}}},"v":{"df":0,"docs":{},"i":{"df":0,"docs":{},"e":{"df":0,"docs":{},"w":{"df":1,"docs":{"2":{"tf":1.0}}}},"s":{"df":0,"docs":{},"u":{"a":{"df":0,"docs":{},"l":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}}},"w":{"df":0,"docs":{},"i":{"d":{"df":0,"docs":{},"e":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}},"x":{"df":1,"docs":{"2":{"tf":1.7320508075688772}}},"y":{"df":1,"docs":{"2":{"tf":1.7320508075688772}}}}},"breadcrumbs":{"root":{"0":{"df":1,"docs":{"2":{"tf":2.0}}},"1":{"4":{"df":1,"docs":{"2":{"tf":1.0}}},"df":1,"docs":{"2":{"tf":2.449489742783178}}},"2":{"6":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"8":{"df":1,"docs":{"1":{"tf":1.0}}},"9":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"df":1,"docs":{"2":{"tf":1.7320508075688772}}},"3":{"1":{"df":1,"docs":{"1":{"tf":1.0}}},"2":{"df":0,"docs":{},"x":{"3":{"2":{"df":1,"docs":{"1":{"tf":1.0}}},"df":0,"docs":{}},"df":0,"docs":{}}},"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"4":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"5":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"6":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"7":{"df":1,"docs":{"2":{"tf":1.7320508075688772}}},"8":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}},"a":{"c":{"df":0,"docs":{},"t":{"df":0,"docs":{},"u":{"a":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}}},"df":0,"docs":{},"l":{"df":0,"docs":{},"w":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}}},"df":0,"docs":{}}},"p":{"df":0,"docs":{},"i":{"df":1,"docs":{"0":{"tf":1.0}}},"p":{"df":0,"docs":{},"e":{"a":{"df":0,"docs":{},"r":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}},"r":{"df":0,"docs":{},"r":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}},"v":{"df":0,"docs":{},"o":{"df":0,"docs":{},"i":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}}}}},"b":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"df":0,"docs":{},"h":{"df":1,"docs":{"2":{"tf":1.0}}},"t":{"df":0,"docs":{},"o":{"df":0,"docs":{},"m":{"df":1,"docs":{"2":{"tf":2.23606797749979}}}}}}}},"c":{"df":0,"docs":{},"e":{"df":0,"docs":{},"l":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.4142135623730951}}}}},"o":{"d":{"df":0,"docs":{},"e":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{},"m":{"df":0,"docs":{},"p":{"df":0,"docs":{},"u":{"df":0,"docs":{},"t":{"df":0,"docs":{},"e":{"d":{"df":0,"docs":{},"g":{"df":0,"docs":{},"r":{"df":0,"docs":{},"i":{"d":{".":{"a":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}},"df":0,"docs":{}},"df":0,"docs":{}}}}},"df":0,"docs":{}}}}}},"n":{"df":0,"docs":{},"f":{"df":0,"docs":{},"u":{"df":0,"docs":{},"s":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"o":{"df":0,"docs":{},"r":{"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"n":{"df":2,"docs":{"1":{"tf":1.0},"2":{"tf":1.4142135623730951}}}}},"df":0,"docs":{}}},"r":{"df":0,"docs":{},"n":{"df":0,"docs":{},"e":{"df":0,"docs":{},"r":{"df":1,"docs":{"2":{"tf":2.0}}}}}}}},"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"m":{"df":0,"docs":{},"e":{"df":0,"docs":{},"n":{"df":0,"docs":{},"s":{"df":0,"docs":{},"i":{"df":0,"docs":{},"o":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}}}}}}}},"r":{"df":0,"docs":{},"e":{"c":{"df":0,"docs":{},"t":{"df":0,"docs":{},"l":{"df":0,"docs":{},"i":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"df":0,"docs":{}}},"s":{"df":0,"docs":{},"p":{"df":0,"docs":{},"l":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}}}},"df":0,"docs":{},"e":{"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"t":{"df":0,"docs":{},"o":{"df":0,"docs":{},"r":{"df":1,"docs":{"2":{"tf":1.0}}}}}}},"df":0,"docs":{}},"g":{"df":0,"docs":{},"h":{"df":0,"docs":{},"o":{"df":0,"docs":{},"s":{"df":0,"docs":{},"t":{"df":1,"docs":{"1":{"tf":1.0}}}}}},"i":{"df":0,"docs":{},"v":{"df":0,"docs":{},"e":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"r":{"df":0,"docs":{},"i":{"d":{"[":{"df":0,"docs":{},"x":{"]":{"[":{"df":0,"docs":{},"i":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}},"df":0,"docs":{}}},"df":3,"docs":{"0":{"tf":1.7320508075688772},"1":{"tf":2.449489742783178},"2":{"tf":2.0}}},"df":0,"docs":{}}}},"h":{"a":{"df":0,"docs":{},"r":{"df":0,"docs":{},"v":{"a":{"df":0,"docs":{},"r":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}}},"df":0,"docs":{}}}},"df":0,"docs":{}},"i":{"df":0,"docs":{},"n":{"d":{"df":0,"docs":{},"e":{"df":0,"docs":{},"x":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}}}},"df":0,"docs":{},"s":{"df":0,"docs":{},"t":{"df":0,"docs":{},"e":{"a":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}},"df":0,"docs":{}}}},"t":{"df":0,"docs":{},"e":{"df":0,"docs":{},"g":{"df":1,"docs":{"1":{"tf":1.0}}}}}}},"l":{"df":0,"docs":{},"e":{"df":0,"docs":{},"f":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":2.23606797749979}}}}},"i":{"df":0,"docs":{},"s":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}},"o":{"df":0,"docs":{},"g":{"df":0,"docs":{},"i":{"c":{"df":1,"docs":{"1":{"tf":1.7320508075688772}}},"df":0,"docs":{}}},"o":{"df":0,"docs":{},"k":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"m":{"a":{"df":0,"docs":{},"t":{"c":{"df":0,"docs":{},"h":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}},"df":0,"docs":{}},"n":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"df":0,"docs":{},"e":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"o":{"df":0,"docs":{},"f":{"df":0,"docs":{},"f":{"df":0,"docs":{},"i":{"c":{"df":0,"docs":{},"i":{"df":1,"docs":{"2":{"tf":1.4142135623730951}}}},"df":0,"docs":{}}}}},"p":{"a":{"c":{"b":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"'":{"df":1,"docs":{"2":{"tf":1.0}}},"df":2,"docs":{"1":{"tf":1.0},"2":{"tf":1.0}}}}},"df":0,"docs":{},"m":{"a":{"df":0,"docs":{},"n":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}},"df":0,"docs":{},"t":{"df":0,"docs":{},"h":{"df":0,"docs":{},"w":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}}}},"df":0,"docs":{},"o":{"df":0,"docs":{},"i":{"df":0,"docs":{},"n":{"df":0,"docs":{},"t":{"2":{"df":1,"docs":{"2":{"tf":1.0}}},"df":1,"docs":{"2":{"tf":1.0}}}}},"s":{"df":0,"docs":{},"i":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}}},"r":{"df":0,"docs":{},"o":{"df":0,"docs":{},"g":{"df":0,"docs":{},"r":{"a":{"df":0,"docs":{},"m":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}}}},"r":{"df":0,"docs":{},"e":{"c":{"df":0,"docs":{},"o":{"df":0,"docs":{},"m":{"df":0,"docs":{},"m":{"df":0,"docs":{},"e":{"df":0,"docs":{},"n":{"d":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{}}}}}}},"df":0,"docs":{},"p":{"df":0,"docs":{},"r":{"df":0,"docs":{},"e":{"df":0,"docs":{},"s":{"df":2,"docs":{"1":{"tf":1.0},"2":{"tf":1.4142135623730951}}}}}}},"i":{"df":0,"docs":{},"g":{"df":0,"docs":{},"h":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":2.23606797749979}}}}}},"o":{"b":{"df":0,"docs":{},"o":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{}},"u":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}},"s":{"df":0,"docs":{},"t":{"df":1,"docs":{"0":{"tf":1.0}}}}}},"s":{"a":{"df":0,"docs":{},"v":{"df":0,"docs":{},"e":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{},"i":{"d":{"df":0,"docs":{},"e":{"df":0,"docs":{},"w":{"a":{"df":0,"docs":{},"y":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}},"df":0,"docs":{}},"p":{"a":{"c":{"df":0,"docs":{},"e":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}},"df":0,"docs":{}},"t":{"a":{"df":0,"docs":{},"r":{"df":0,"docs":{},"t":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{}},"u":{"df":0,"docs":{},"p":{"df":0,"docs":{},"p":{"df":0,"docs":{},"o":{"df":0,"docs":{},"r":{"df":0,"docs":{},"t":{"df":1,"docs":{"1":{"tf":1.0}}}}}}}}},"t":{"a":{"df":0,"docs":{},"l":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.0}}}}},"df":0,"docs":{},"o":{"df":0,"docs":{},"p":{"df":1,"docs":{"2":{"tf":2.0}}}},"r":{"a":{"df":0,"docs":{},"v":{"df":0,"docs":{},"e":{"df":0,"docs":{},"l":{"df":1,"docs":{"1":{"tf":1.0}}}}}},"df":0,"docs":{}}},"u":{"8":{"df":1,"docs":{"2":{"tf":1.0}}},"df":0,"docs":{},"p":{"df":1,"docs":{"2":{"tf":1.0}}},"s":{"df":1,"docs":{"2":{"tf":1.0}}}},"v":{"df":0,"docs":{},"i":{"df":0,"docs":{},"e":{"df":0,"docs":{},"w":{"df":1,"docs":{"2":{"tf":1.0}}}},"s":{"df":0,"docs":{},"u":{"a":{"df":0,"docs":{},"l":{"df":1,"docs":{"2":{"tf":1.0}}}},"df":0,"docs":{}}}}},"w":{"df":0,"docs":{},"i":{"d":{"df":0,"docs":{},"e":{"df":1,"docs":{"1":{"tf":1.0}}}},"df":0,"docs":{}}},"x":{"df":1,"docs":{"2":{"tf":1.7320508075688772}}},"y":{"df":1,"docs":{"2":{"tf":1.7320508075688772}}}}},"title":{"root":{"c":{"df":0,"docs":{},"o":{"df":0,"docs":{},"o":{"df":0,"docs":{},"r":{"d":{"df":0,"docs":{},"i":{"df":0,"docs":{},"n":{"df":1,"docs":{"2":{"tf":1.0}}}}},"df":0,"docs":{}}}}},"df":0,"docs":{},"g":{"df":0,"docs":{},"r":{"df":0,"docs":{},"i":{"d":{"df":2,"docs":{"0":{"tf":1.0},"1":{"tf":1.0}}},"df":0,"docs":{}}}},"l":{"df":0,"docs":{},"o":{"df":0,"docs":{},"g":{"df":0,"docs":{},"i":{"c":{"df":1,"docs":{"1":{"tf":1.0}}},"df":0,"docs":{}}}}}}}},"lang":"English","pipeline":["trimmer","stopWordFilter","stemmer"],"ref":"id","version":"0.9.5"},"results_options":{"limit_results":30,"teaser_word_count":30},"search_options":{"bool":"OR","expand":true,"fields":{"body":{"boost":1},"breadcrumbs":{"boost":1},"title":{"boost":2}}}});