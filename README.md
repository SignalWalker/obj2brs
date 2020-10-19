# obj2brs

a5 adaptation of [textured-voxelizer](https://github.com/CheezBarger/textured-voxelizer) by French Fries

![Voxelized plane](https://github.com/CheezBarger/textured-voxelizer/blob/master/banner.png)

Generates textured voxel models from OBJ files.
Currently only supports voxelization and simplification for BRS files.

```
USAGE:
    cargo run --release <file> <output> --bricktype <bricktype> --scale <scale> --simplify <simplify>
```

The program supports two color modes when simplifying: lossless, and lossy. Lossless will prioritize color accuracy, while lossy will prioritize brick count.