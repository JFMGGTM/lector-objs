use std::{fs, path::Path, str::FromStr};

#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub u: f32,
    pub v: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Index {
    pub v: Option<i32>,  // índice a posición 
    pub vt: Option<i32>, // índice a coordenadas de textura
    pub vn: Option<i32>, // índice a normal
}

#[derive(Debug, Clone)]
pub struct Face {
    pub indices: Vec<Index>, // una cara puede traer 3 o más vértices
    pub group: Option<String>,
    pub usemtl: Option<String>,
    pub smoothing: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub normals:   Vec<Vec3>,
    pub faces:     Vec<Face>,
    pub objects:   Vec<String>,
    pub groups:    Vec<String>, 
    pub mtllibs:   Vec<String>, // referencias a materiales
}


fn resolve_idx(raw: i32, len: usize) -> Option<usize> {
    if len == 0 { return None; }
    if raw > 0 {
        let i = (raw as usize).checked_sub(1)?;
        if i < len { Some(i) } else { None }
    } else {
        // NOTA: negativo -1 =  último
        let i = (len as i32 + raw) as isize;
        if i >= 0 && (i as usize) < len { Some(i as usize) } else { None }
    }
}

impl FromStr for Vec3 {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split_whitespace();
        let x = it.next().ok_or("Vec3 x")?.parse::<f32>().map_err(|e| e.to_string())?;
        let y = it.next().ok_or("Vec3 y")?.parse::<f32>().map_err(|e| e.to_string())?;
        let z = it.next().ok_or("Vec3 z")?.parse::<f32>().map_err(|e| e.to_string())?;
        Ok(Vec3 { x, y, z })
    }
}

impl FromStr for Vec2 {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split_whitespace();
        let u = it.next().ok_or("Vec2 u")?.parse::<f32>().map_err(|e| e.to_string())?;
        let v = it.next().ok_or("Vec2 v")?.parse::<f32>().map_err(|e| e.to_string())?;
        Ok(Vec2 { u, v })
    }
}


fn parse_index(token: &str) -> Result<Index, String> {
    let mut parts = token.split('/');
    let v  = parts.next().unwrap_or("").trim();
    let vt = parts.next().unwrap_or("").trim();
    let vn = parts.next().unwrap_or("").trim();

    let v  = if v.is_empty()  { None } else { Some(v.parse::<i32>().map_err(|e| e.to_string())?) };
    let vt = if vt.is_empty() { None } else { Some(vt.parse::<i32>().map_err(|e| e.to_string())?) };
    let vn = if vn.is_empty() { None } else { Some(vn.parse::<i32>().map_err(|e| e.to_string())?) };

    Ok(Index { v, vt, vn })
}

/// Convierte el texto de un .obj a una malla en memoria.
pub fn parse_obj(text: &str) -> Result<Mesh, String> {
    let mut mesh = Mesh::default();

    let mut current_group: Option<String> = None;
    let mut current_mtl:   Option<String> = None;
    let mut current_s:     Option<String> = None;

    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let tag = it.next().unwrap_or("");

        match tag {
            "v" => {
                // Lee 3 números (x y z)
                let coords = it.collect::<Vec<_>>().join(" ");
                let v = Vec3::from_str(&coords)
                    .map_err(|e| format!("L{}: 'v' inválido: {}", lineno + 1, e))?;
                mesh.positions.push(v);
            }
            "vt" => {
                let vals: Vec<&str> = it.collect();
                if vals.len() < 2 {
                    return Err(format!("L{}: 'vt' necesita al menos 2 valores", lineno + 1));
                }
                let u = vals[0].parse::<f32>().map_err(|e| e.to_string())?;
                let v = vals[1].parse::<f32>().map_err(|e| e.to_string())?;
                mesh.texcoords.push(Vec2 { u, v });
            }
            "vn" => {
                // Normal en 3D
                let coords = it.collect::<Vec<_>>().join(" ");
                let n = Vec3::from_str(&coords)
                    .map_err(|e| format!("L{}: 'vn' inválido: {}", lineno + 1, e))?;
                mesh.normals.push(n);
            }
            "f" => {
                // Cara con 3 o más vértices. Cada vértice puede traer v/vt/vn.
                let mut idxs = Vec::new();
                for tok in it {
                    let idx = parse_index(tok)
                        .map_err(|e| format!("L{}: índice de cara inválido '{}': {}", lineno + 1, tok, e))?;
                    idxs.push(idx);
                }
                if idxs.len() < 3 {
                    return Err(format!("L{}: una cara necesita 3 o más vértices", lineno + 1));
                }
                mesh.faces.push(Face {
                    indices: idxs,
                    group: current_group.clone(),
                    usemtl: current_mtl.clone(),
                    smoothing: current_s.clone(),
                });
            }
            "o" => {
                let name = it.collect::<Vec<_>>().join(" ");
                if !name.is_empty() {
                    mesh.objects.push(name);
                }
            }
            "g" => {
                let gname = it.collect::<Vec<_>>().join(" ");
                current_group = if gname.is_empty() { None } else { Some(gname.clone()) };
                if let Some(g) = current_group.clone() {
                    mesh.groups.push(g);
                }
            }
            "s" => {
                // Estado de “suavizado” 
                let sval = it.collect::<Vec<_>>().join(" ");
                current_s = if sval.is_empty() { None } else { Some(sval) };
            }
            "mtllib" => {
                // Nombre de archivo .mtl referido.
                let mtllib = it.collect::<Vec<_>>().join(" ");
                if !mtllib.is_empty() {
                    mesh.mtllibs.push(mtllib);
                }
            }
            "usemtl" => {
                // Material actual (texto)
                let m = it.collect::<Vec<_>>().join(" ");
                current_mtl = if m.is_empty() { None } else { Some(m) };
            }
            // Si aparece algo que no ocupamos, lo dejamos pasar sin dar error.
            _ => {}
        }
    }

    Ok(mesh)
}

/// Índices ya “resueltos” a 0-based. Si algo no venía en el archivo, queda en None.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedIndex {
    pub v: Option<usize>,
    pub vt: Option<usize>,
    pub vn: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Triangle {
    pub a: ResolvedIndex,
    pub b: ResolvedIndex,
    pub c: ResolvedIndex,
    pub group: Option<String>,
    pub usemtl: Option<String>,
    pub smoothing: Option<String>,
}

/// Convierte un Index según el largo de cada lista
fn resolve_index(idx: &Index, vlen: usize, vtlen: usize, vnlen: usize) -> ResolvedIndex {
    ResolvedIndex {
        v:  idx.v.and_then(|i| resolve_idx(i, vlen)),
        vt: idx.vt.and_then(|i| resolve_idx(i, vtlen)),
        vn: idx.vn.and_then(|i| resolve_idx(i, vnlen)),
    }
}

/// Convierte todas las caras a triángulos.
pub fn triangulate(mesh: &Mesh) -> Vec<Triangle> {
    let mut out = Vec::new();
    for f in &mesh.faces {
        if f.indices.len() == 3 {
            let a = resolve_index(&f.indices[0], mesh.positions.len(), mesh.texcoords.len(), mesh.normals.len());
            let b = resolve_index(&f.indices[1], mesh.positions.len(), mesh.texcoords.len(), mesh.normals.len());
            let c = resolve_index(&f.indices[2], mesh.positions.len(), mesh.texcoords.len(), mesh.normals.len());
            out.push(Triangle { a, b, c, group: f.group.clone(), usemtl: f.usemtl.clone(), smoothing: f.smoothing.clone() });
        } else {
            // Abanico: (0, i, i+1)
            for i in 1..(f.indices.len() - 1) {
                let a = resolve_index(&f.indices[0], mesh.positions.len(), mesh.texcoords.len(), mesh.normals.len());
                let b = resolve_index(&f.indices[i], mesh.positions.len(), mesh.texcoords.len(), mesh.normals.len());
                let c = resolve_index(&f.indices[i + 1], mesh.positions.len(), mesh.texcoords.len(), mesh.normals.len());
                out.push(Triangle { a, b, c, group: f.group.clone(), usemtl: f.usemtl.clone(), smoothing: f.smoothing.clone() });
            }
        }
    }
    out
}

/// Lee un .obj desde disco y lo devuelve como Mesh.
pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<Mesh, String> {
    let data = fs::read_to_string(&path).map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
    parse_obj(&data)
}

fn main() {
    //  NOTA: Para correr usar  cargo run --release -- ruta/al/modelo.obj
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Uso: obj_reader <ruta/al/modelo.obj>");
        std::process::exit(1);
    });

    match load_obj(&path) {
        Ok(mesh) => {
            println!("== OBJ cargado ==");
            println!("Posiciones: {}", mesh.positions.len());
            println!("Texcoords:  {}", mesh.texcoords.len());
            println!("Normales:   {}", mesh.normals.len());
            println!("Caras (tal como vienen): {}", mesh.faces.len());

            let tris = triangulate(&mesh);
            println!("Triángulos (después de convertir): {}", tris.len());

            println!("== Previsualización de los primeros 3 triangulos ==");
            for (i, t) in tris.iter().take(3).enumerate() {
                let fmt = |ri: Option<usize>| ri.map(|x| x.to_string()).unwrap_or_else(|| "-".into());
                println!(
                    "Tri {}: v({},{},{}) vt({},{},{}) vn({},{},{})",
                    i,
                    fmt(t.a.v),  fmt(t.b.v),  fmt(t.c.v),
                    fmt(t.a.vt), fmt(t.b.vt), fmt(t.c.vt),
                    fmt(t.a.vn), fmt(t.b.vn), fmt(t.c.vn),
                );
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
