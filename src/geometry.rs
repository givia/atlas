use crate::vertex::Vertex;
use geo::{
    BooleanOps, BoundingRect, Coord, CoordsIter, Geometry, GeometryCollection, LineString, Polygon,
    TriangulateEarcut, interior_point,
};
use geojson::{FeatureCollection, GeoJson};
// use geojson::{feature, FeatureCollection, GeoJson, Geometry};
use rand::rand_core::le;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Read;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::{thread, vec};
use wgpu::wgc::error::MultiError;

use mvt_reader::Reader;
use protobuf::{EnumOrUnknown, Message};

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use geo_types::Point;

const EARTH_RADIUS: f64 = 6_378_137.0;
const BLUE_COLOR: [f32; 4] = [0.0, 0.3, 1.0, 1.0];

pub fn convert(lon: f64, lat: f64, radius: f64) -> [f32; 3] {
    let phi = lat.to_radians();
    let theta = lon.to_radians();

    let x = -(radius * phi.sin() * theta.cos());
    let y = radius * phi.cos();
    let z = radius * phi.sin() * theta.sin();

    [x as f32, y as f32, z as f32]
}

pub fn convert32(lon: f32, lat: f32, radius: f32) -> [f32; 3] {
    let phi = lat.to_radians();
    let theta = lon.to_radians();

    let x = -(radius * phi.sin() * theta.cos());
    let y = radius * phi.cos();
    let z = radius * phi.sin() * theta.sin();

    [x as f32, y as f32, z as f32]
}

/// Generates a UV sphere mesh with radius 6371, 360 segments, and 180 rings.
pub fn generate_sphere() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices: Vec<Vertex> = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut index = 0;

    for x in 0..=360 {
        for y in 0..=180 {
            let point = Point::new(x as f64, y as f64);
            let point2 = Point::new(x as f64 + 1.0, y as f64 + 1.0);

            let rect = Polygon::new(
                LineString::from(vec![
                    (point.x(), point.y()),
                    (point2.x(), point.y()),
                    (point2.x(), point2.y()),
                    (point.x(), point2.y()),
                    (point.x(), point.y()),
                ]),
                vec![],
            );
            rect.earcut_triangles().iter().for_each(|i| {
                vertices.push(Vertex {
                    position: convert(i.0.x, i.0.y, EARTH_RADIUS),
                    color: BLUE_COLOR,
                });
                indices.push(index);
                index += 1;

                vertices.push(Vertex {
                    position: convert(i.1.x, i.1.y, EARTH_RADIUS),
                    color: BLUE_COLOR,
                });
                indices.push(index);
                index += 1;

                vertices.push(Vertex {
                    position: convert(i.2.x, i.2.y, EARTH_RADIUS),
                    color: BLUE_COLOR,
                });
                indices.push(index);
                index += 1;
            });
        }
    }

    (vertices, indices)
}

pub fn draw_polygon(polygon: &Vec<Vec<Vec<f64>>>, color: [f32; 4]) -> (Vec<Vertex>, Vec<u32>) {
    let mut index = 0;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let pg = Polygon::new(
        polygon
            .iter()
            .take(1)
            .flat_map(|ring| {
                ring.iter()
                    .map(|coord| Point::new(coord[0] + 180.0, 90.0 - coord[1]))
                    .collect::<LineString<_>>()
            })
            .collect(),
        polygon
            .iter()
            .skip(1)
            .map(|ring| {
                ring.iter()
                    .map(|coord| Point::new(coord[0] + 180.0, 90.0 - coord[1]))
                    .collect::<LineString<_>>()
            })
            .collect(),
    );

    match pg.bounding_rect() {
        Some(rect) => {
            let min = rect.min();
            let max = rect.max();

            let min_x = min.x.floor() as i32;
            let min_y = min.y.floor() as i32;
            let max_x = max.x.ceil() as i32;
            let max_y = max.y.ceil() as i32;

            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    let point1 = Point::new(x as f64, y as f64);
                    let point2 = Point::new((x + 1) as f64, y as f64);
                    let point3 = Point::new(x as f64, (y + 1) as f64);
                    let point4 = Point::new((x + 1) as f64, (y + 1) as f64);

                    let rect = Polygon::new(
                        LineString::from(vec![
                            (point1.x(), point1.y()),
                            (point2.x(), point2.y()),
                            (point4.x(), point4.y()),
                            (point3.x(), point3.y()),
                            (point1.x(), point1.y()),
                        ]),
                        vec![],
                    );

                    pg.intersection(&rect).0.iter().for_each(|polygon| {
                        polygon.earcut_triangles().iter().for_each(|i| {
                            vertices.push(Vertex {
                                position: convert(i.0.x, i.0.y, EARTH_RADIUS),
                                color: color,
                            });
                            indices.push(index);
                            index += 1;

                            vertices.push(Vertex {
                                position: convert(i.1.x, i.1.y, EARTH_RADIUS),
                                color: color,
                            });
                            indices.push(index);
                            index += 1;

                            vertices.push(Vertex {
                                position: convert(i.2.x, i.2.y, EARTH_RADIUS),
                                color: color,
                            });
                            indices.push(index);
                            index += 1;
                        });
                    });
                }
            }
            (vertices, indices)
        }
        None => (vertices, indices),
    }
}

pub fn draw_polygon_p(polygon: Polygon<f32>, color: [f32; 4]) -> (Vec<Vertex>, Vec<u32>) {
    let mut index = 0;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    match polygon.bounding_rect() {
        Some(rect) => {
            let min = rect.min();
            let max = rect.max();

            let min_x = min.x.floor() as i32;
            let min_y = min.y.floor() as i32;
            let max_x = max.x.ceil() as i32;
            let max_y = max.y.ceil() as i32;

            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    let point1 = Point::new(x as f32, y as f32);
                    let point2 = Point::new((x + 1) as f32, y as f32);
                    let point3 = Point::new(x as f32, (y + 1) as f32);
                    let point4 = Point::new((x + 1) as f32, (y + 1) as f32);

                    let rect = Polygon::new(
                        LineString::from(vec![
                            (point1.x(), point1.y()),
                            (point2.x(), point2.y()),
                            (point4.x(), point4.y()),
                            (point3.x(), point3.y()),
                            (point1.x(), point1.y()),
                        ]),
                        vec![],
                    );

                    polygon.intersection(&rect).0.iter().for_each(|polygon| {
                        polygon.earcut_triangles().iter().for_each(|i| {
                            vertices.push(Vertex {
                                position: convert32(i.0.x, i.0.y, EARTH_RADIUS as f32),
                                color: color,
                            });
                            indices.push(index);
                            index += 1;

                            vertices.push(Vertex {
                                position: convert32(i.1.x, i.1.y, EARTH_RADIUS as f32),
                                color: color,
                            });
                            indices.push(index);
                            index += 1;

                            vertices.push(Vertex {
                                position: convert32(i.2.x, i.2.y, EARTH_RADIUS as f32),
                                color: color,
                            });
                            indices.push(index);
                            index += 1;
                        });
                    });
                }
            }
            (vertices, indices)
        }
        None => (vertices, indices),
    }
}

pub async fn load_tile(sender: Sender<(Vec<Vertex>, Vec<u32>)>, z: u32, x: u32, y: u32) {
    // Load the tile data for the specified zoom level and coordinates

    let random_color = [1.0, 1.0, 0.0, 1.0];

    let tile_url = format!(
        "https://api.mapbox.com/v4/mapbox.mapbox-streets-v8/{}/{}/{}.mvt?access_token=pk.eyJ1IjoiZ2l2aWEiLCJhIjoiY21lemt1MHBvMTAxaTJqczdicWFwendlMiJ9.EBUXn3B7aQoxTBgFoOy3sA",
        z, x, y
    );

    let scale = 2u32.pow(z);

    let tile_x = x as f64 * (360.0 / scale as f64);
    let tile_y = y as f64 * (180.0 / scale as f64);

    let bytes = reqwest::get(&tile_url)
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    println!("Bytes len: {:?}", bytes.len());
    let reader = Reader::new(bytes.to_vec()).unwrap();

    // Print layer metadata
    let layers = reader.get_layer_metadata().unwrap();
    for layer in layers {
        println!("Layer metadata - extent: {:?}", layer.extent);
        println!("Layer metadata - min zoom: {:?}", layer.feature_count);
        println!("Layer metadata - max zoom: {:?}", layer.layer_index);
        println!("Layer metadata - name: {:?}", layer.name);
        println!("Layer version: {:?}", layer.version);

        let layer_index = layer.layer_index;
        let extent = layer.extent;
        let features = reader.get_features(layer_index).unwrap();
        for feature in features {

                    // Handle polygon geometry
                    let mut coords = feature
                        .geometry
                        .coords_iter()
                        .map(|coord| {
                            let x = coord.x as f64 / extent as f64;
                            let y = coord.y as f64 / extent as f64;

                            let lon = 180.0 + tile_x + x * (360.0 / scale as f64);
                            let lat = 90.0 - tile_y + y * (180.0 / scale as f64);
                            println!("Lon: {}, Lat: {}", lon, lat);
                            vec![lon, lat]
                        })
                        .collect::<Vec<_>>();

                    coords.push(coords[0].clone());

                    draw_polygon(&vec![coords], random_color);

            
        }
    }
}

pub fn generate_mesh(sender: Sender<(Vec<Vertex>, Vec<u32>)>) {
    // let (mut vertices, mut indices) = generate_sphere();
    // let mut index = vertices.len() as u32;

    let geojson_string = std::fs::read_to_string("countries.geojson").unwrap();
    let json: GeoJson = geojson_string.parse::<GeoJson>().unwrap();
    let feature_collection: FeatureCollection = FeatureCollection::try_from(json).unwrap();

    let random_color = [1.0, 1.0, 0.0, 1.0];

    for feature in feature_collection.features {
        if let Some(geometry) = feature.geometry {
            match geometry.value {
                geojson::Value::MultiPolygon(multi_polygon) => {
                    multi_polygon.iter().for_each(|polygon| {
                        let polygon_clone = polygon.clone();
                        let sender_clone = sender.clone();
                        tokio::spawn(async move {
                            let (vs, is) = draw_polygon(&polygon_clone, random_color);
                            sender_clone.send((vs, is)).unwrap();
                        });
                    });
                }

                geojson::Value::Polygon(polygon) => {
                    let sender_clone = sender.clone();
                    tokio::spawn(async move {
                        let (vs, is) = draw_polygon(&polygon, random_color);
                        sender_clone.send((vs, is)).unwrap();
                    });
                }
                geojson::Value::Point(_items) => todo!(),
                geojson::Value::MultiPoint(_items) => todo!(),
                geojson::Value::LineString(_items) => todo!(),
                geojson::Value::MultiLineString(_items) => todo!(),
                geojson::Value::GeometryCollection(_items) => todo!(),
            };
        }
    }
}
